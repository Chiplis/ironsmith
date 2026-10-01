//! Canonical optional evidence, ward, bestow and squad cost-pair audit.
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, OrderContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, Phase, PlayerId, Zone};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    actor: PlayerId,
    targets: Vec<Target>,
    stage: String,
    trace: Vec<Value>,
    chosen: Vec<ObjectId>,
    reverse: bool,
    x: u32,
    plot: bool,
    option_text: String,
    optional_repeat_capacity: Option<(bool, Option<u32>)>,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","context":format!("{c:?}"),"selected":self.plot}));
        self.plot
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        if c.requirements.len() > 1 {
            for (i, r) in c.requirements.iter().enumerate() {
                assert_eq!(r.min_targets, 1);
                assert!(r.legal_targets.contains(&self.targets[i]));
            }
        } else {
            let r = &c.requirements[0];
            assert!(
                self.targets.len() >= r.min_targets
                    && r.max_targets.is_none_or(|m| self.targets.len() <= m)
            );
            assert!(self.targets.iter().all(|t| r.legal_targets.contains(t)));
        }
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{:?}",self.targets)}));
        self.targets.clone()
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = if c.description.starts_with("Choose optional costs") {
            self.optional_repeat_capacity = c.options.first().map(|o| (o.repeatable, o.max_count));
            if self.plot {
                c.options
                    .iter()
                    .filter(|o| o.legal)
                    .take(1)
                    .map(|o| o.index)
                    .collect()
            } else {
                vec![]
            }
        } else if !self.option_text.is_empty()
            && !c.description.contains("Confirm mana")
            && c.options
                .iter()
                .any(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
        {
            vec![
                c.options
                    .iter()
                    .find(|o| o.legal && o.description.to_lowercase().contains(&self.option_text))
                    .unwrap()
                    .index,
            ]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace.push(json!({"stage":self.stage,"choice":"options","context":format!("{c:?}"),"selected":selected}));
        selected
    }
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x.max(c.min).min(c.max)}));
        self.x.max(c.min).min(c.max)
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if self.stage == "resolve_Deadly Cover-Up" {
            c.candidates
                .iter()
                .filter(|o| {
                    o.legal
                        && o.name == "Hill Giant"
                        && g.object(o.id).is_some_and(|x| x.owner == PlayerId(1))
                })
                .take(c.max.unwrap_or(usize::MAX))
                .map(|o| o.id)
                .collect::<Vec<_>>()
        } else if self.stage == "resolve_Analyze the Pollen" {
            c.candidates
                .iter()
                .filter(|o| o.legal && o.name == if self.plot { "Grizzly Bears" } else { "Forest" })
                .take(1)
                .map(|o| o.id)
                .collect::<Vec<_>>()
        } else {
            self.chosen
                .iter()
                .copied()
                .filter(|id| c.candidates.iter().any(|o| o.legal && o.id == *id))
                .take(c.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>()
        };
        assert!(
            selected.len() >= c.min.min(c.candidates.iter().filter(|o| o.legal).count()),
            "selection fixture: {c:?}, requested={:?}, selected={selected:?}",
            self.chosen
        );
        self.trace.push(json!({"stage":self.stage,"choice":"objects","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
        selected
    }
    fn decide_order(&mut self, g: &GameState, c: &OrderContext) -> Vec<ObjectId> {
        let mut selected = c.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        if c.description.to_lowercase().contains("library") {
            selected.sort_by_key(|id| {
                g.object(*id)
                    .map(|o| o.name.to_string())
                    .unwrap_or_default()
            });
            if self.reverse {
                selected.reverse();
            }
        }
        self.trace.push(json!({"stage":self.stage,"choice":"order","context":format!("{c:?}"),"selected":selected.iter().map(|id|g.object(*id).map(|o|o.name.to_string())).collect::<Vec<_>>()}));
        selected
    }
    fn view_cards(
        &mut self,
        g: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        c: &ViewCardsContext,
    ) {
        self.trace.push(json!({"stage":self.stage,"choice":"view","context":format!("{c:?}"),"viewer":viewer.0,"cards":cards.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}));
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    g.set_random_seed(SEED);
    g.turn.turn_number = 3;
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g.mark_main_phase_started();
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        for player in [alice(), PlayerId(1)] {
            g.player_mut(player).unwrap().mana_pool.add(s, 30);
        }
    }
    g
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
    let mut state = PriorityLoopState::new(g.players_in_game());
    let initial = g.stack.len();
    dm.trace
        .push(json!({"stage":dm.stage,"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && g.stack.len() > initial
        {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(c) = progress else {
            return Err(format!("announcement stopped:{progress:?}"));
        };
        if matches!(c, DecisionContext::Priority(_)) {
            return Err("announcement returned priority before ability/spell stacked".into());
        }
        progress =
            apply_decision_context_with_dm(g, q, &mut state, &c, dm).map_err(|e| e.to_string())?;
    }
    Err("announcement budget".into())
}
fn one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
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
    check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
    advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
    Ok(())
}
fn finish(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Dm) -> Result<(), String> {
    for _ in 0..24 {
        check_and_apply_sbas_with(g, q, dm).map_err(|e| e.to_string())?;
        advance_priority_with_dm(g, q, dm).map_err(|e| e.to_string())?;
        if g.stack.is_empty() {
            return Ok(());
        }
        one(g, q, dm)?;
    }
    Err("resolution budget".into())
}
fn cast_announce(
    g: &mut GameState,
    d: &CardDefinition,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(dm.actor);
    let id = g.create_object_from_definition(d, dm.actor, Zone::Hand);
    let a = compute_legal_actions(g, dm.actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(dm.actor).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(dm.actor).unwrap().mana_pool.total();
    dm.trace
        .push(json!({"stage":"actual_paid_cast","card":d.name(),"paid":paid}));
    if paid != cost {
        return Err(format!(
            "fixture {} expected castcost {cost}, paid {paid}",
            d.name()
        ));
    }
    Ok(())
}

fn paid_cast(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    n: &str,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<ironsmith::ids::StableId, String> {
    let before = g
        .objects_in_deterministic_order()
        .into_iter()
        .map(|o| o.stable_id)
        .collect::<Vec<_>>();
    dm.stage = format!("paid_cast_{n}");
    let definition = &defs
        .get(n)
        .ok_or_else(|| format!("canonical helper did not compile: {n}"))?
        .0;
    cast_announce(g, definition, cost, q, dm)?;
    dm.stage = format!("resolve_{n}");
    finish(g, q, dm)?;
    g.objects_in_deterministic_order()
        .into_iter()
        .find(|o| o.name == defs[n].0.name() && !before.contains(&o.stable_id))
        .map(|o| o.stable_id)
        .ok_or(format!("cast resource {n} absent after resolution"))
}
fn current(g: &GameState, id: ironsmith::ids::StableId) -> ObjectId {
    g.find_object_by_stable_id(id)
        .expect("current resource incarnation")
}

fn set_actor(g: &mut GameState, dm: &mut Dm, p: PlayerId) {
    dm.actor = p;
    g.turn.active_player = p;
    g.turn.priority_player = Some(p);
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
}
fn zone(g: &GameState, id: ironsmith::ids::StableId) -> String {
    format!("{:?}", g.object(current(g, id)).unwrap().zone)
}
fn named(g: &GameState, id: ObjectId, name: &str) -> u32 {
    g.object(id)
        .unwrap()
        .counters
        .iter()
        .filter(|(k, _)| k.description().eq_ignore_ascii_case(name))
        .map(|(_, n)| *n)
        .sum()
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let v = c["variant"].as_str().unwrap();
    let accept = !["insufficient", "decline"].contains(&v);
    let mut g = game();
    let mut q = TriggerQueue::new();
    for p in [alice(), PlayerId(1), PlayerId(2)] {
        for _ in 0..12 {
            g.create_object_from_definition(&defs["Forest"].0, p, Zone::Library);
        }
    }
    g.create_object_from_definition(&defs["Grizzly Bears"].0, alice(), Zone::Library);
    let witness = paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?;
    let mut opponents = vec![];
    if [
        "Bite Down on Crime",
        "Crimestopper Sprite",
        "Deadly Cover-Up",
        "Extract a Confession",
    ]
    .contains(&n)
    {
        for p in [PlayerId(1), PlayerId(2)] {
            set_actor(&mut g, dm, p);
            for col in [ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
                g.player_mut(p).unwrap().mana_pool.add(col, 10);
            }
            let small = paid_cast(&mut g, defs, "Grizzly Bears", 2, &mut q, dm)?;
            let big = paid_cast(&mut g, defs, "Hill Giant", 4, &mut q, dm)?;
            opponents.push((small, big));
        }
        set_actor(&mut g, dm, alice());
    }
    let source_pre = if n == "Axebane Ferox" {
        Some(paid_cast(&mut g, defs, n, 4, &mut q, dm)?)
    } else {
        None
    };
    let payer = if n == "Axebane Ferox" {
        PlayerId(1)
    } else {
        alice()
    };
    set_actor(&mut g, dm, payer);
    let mut materials = vec![];
    let material_names = if n == "Ruthless Radrat" {
        vec![
            "Forest";
            if v == "insufficient" {
                2
            } else if v == "surplus" {
                7
            } else {
                3
            }
        ]
    } else if n == "Axebane Ferox" {
        vec![if v == "insufficient" {
            "Grizzly Bears"
        } else if v == "surplus" {
            "Craw Wurm"
        } else {
            "Hill Giant"
        }]
    } else if n == "Analyze the Pollen" {
        if v == "insufficient" {
            vec!["Craw Wurm"]
        } else if v == "surplus" {
            vec!["Craw Wurm", "Hill Giant"]
        } else {
            vec!["Craw Wurm", "Grizzly Bears"]
        }
    } else if v == "insufficient" {
        vec!["Hill Giant"]
    } else if v == "surplus" {
        vec!["Craw Wurm", "Grizzly Bears"]
    } else {
        vec!["Craw Wurm"]
    };
    for name in material_names {
        let id = g.create_object_from_definition(&defs[name].0, payer, Zone::Hand);
        materials.push(g.object(id).unwrap().stable_id);
    }
    let grave_source = if v == "graveyard_exact" {
        let id = g.create_object_from_definition(&defs[n].0, alice(), Zone::Hand);
        Some(g.object(id).unwrap().stable_id)
    } else {
        None
    };
    let producer = paid_cast(&mut g, defs, "One with Nothing", 1, &mut q, dm)?;
    if n == "Ruthless Radrat" {
        materials.push(producer)
    }
    if materials.iter().any(|s| zone(&g, *s) != "Graveyard") {
        return Err("paid discard resource producer failed".into());
    }
    dm.plot = accept;
    dm.x = if n == "Ruthless Radrat" && v == "surplus" {
        2
    } else {
        1
    };
    dm.chosen = if accept {
        materials.iter().map(|s| current(&g, *s)).collect()
    } else {
        vec![]
    };
    dm.targets = match n {
        "Bite Down on Crime" => vec![
            Target::Object(current(&g, witness)),
            Target::Object(current(&g, opponents[0].1)),
        ],
        "Crimestopper Sprite" => vec![Target::Object(current(&g, opponents[0].0))],
        "Vitu-Ghazi Inspector" => {
            if accept {
                vec![Target::Object(current(&g, witness))]
            } else {
                vec![]
            }
        }
        "Detective's Phoenix" => {
            if v == "decline" {
                vec![]
            } else {
                vec![Target::Object(current(&g, witness))]
            }
        }
        "Axebane Ferox" => vec![Target::Object(current(&g, source_pre.unwrap()))],
        _ => vec![],
    };
    let spell_name = if n == "Axebane Ferox" { "Murder" } else { n };
    let source = if let Some(s) = grave_source {
        s
    } else {
        let id = g.create_object_from_definition(&defs[spell_name].0, payer, Zone::Hand);
        g.object(id).unwrap().stable_id
    };
    let id = current(&g, source);
    let bestow = n == "Detective's Phoenix" && v != "decline";
    let action=compute_legal_actions(&g,payer).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==id&&if bestow{matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(0)|ironsmith::alternative_cast::CastingMethod::PlayFrom{use_alternative:Some(0),..})}else{matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Normal)}));
    let wanted_action = !(bestow && v == "insufficient");
    dm.trace.push(json!({"stage":"before_cost_probe","resources":materials.iter().map(|s|json!({"name":g.object(current(&g,*s)).unwrap().name.to_string(),"zone":zone(&g,*s)})).collect::<Vec<_>>(),"source_zone":zone(&g,source),"action":format!("{action:?}"),"accept":accept}));
    if action.is_none() || !wanted_action {
        return Ok((
            json!({"cast_offered":wanted_action}),
            json!({"cast_offered":action.is_some()}),
        ));
    }
    dm.stage = format!("announce_{n}");
    let before = g.player(payer).unwrap().mana_pool.total();
    let announce_error = announce(&mut g, action.unwrap(), &mut q, dm).err();
    let paid = before - g.player(payer).unwrap().mana_pool.total();
    let printed = match n {
        "Analyze the Pollen" => 1,
        "Axebane Ferox" => 3,
        "Bite Down on Crime" => {
            if accept {
                2
            } else {
                4
            }
        }
        "Crimestopper Sprite" | "Ruthless Radrat" => 3,
        "Deadly Cover-Up" => 5,
        "Detective's Phoenix" => {
            if bestow {
                1
            } else {
                3
            }
        }
        "Extract a Confession" | "Vitu-Ghazi Inspector" => 2,
        _ => unreachable!(),
    };
    let exiled_before = materials
        .iter()
        .filter(|s| zone(&g, **s) == "Exile")
        .count();
    let expect_exiled_before = if accept && n != "Axebane Ferox" {
        if n == "Ruthless Radrat" {
            4
        } else {
            materials.len()
        }
    } else {
        0
    };
    let expected_cost = json!({"error":null,"paid":printed,"resources_exiled":expect_exiled_before,"source_on_stack":true});
    let actual_cost = json!({"error":announce_error,"paid":paid,"resources_exiled":exiled_before,"source_on_stack":zone(&g,source)=="Stack"});
    dm.trace.push(
        json!({"stage":"actual_announcement_cost","expected":expected_cost,"actual":actual_cost}),
    );
    if actual_cost != expected_cost {
        return Ok((expected_cost, actual_cost));
    }
    if n == "Extract a Confession" {
        dm.chosen = opponents
            .iter()
            .map(|(small, big)| current(&g, if accept { *big } else { *small }))
            .collect();
    }
    dm.stage = format!("resolve_{n}");
    let error = finish(&mut g, &mut q, dm).err();
    let sid = current(&g, source);
    let wid = current(&g, witness);
    let expected = match n {
        "Analyze the Pollen" => json!({"hand_names":[if accept{"Grizzly Bears"}else{"Forest"}]}),
        "Axebane Ferox" => {
            json!({"ferox_zone":if accept{"Graveyard"}else{"Battlefield"},"resources_exiled":if accept{materials.len()}else{0}})
        }
        "Bite Down on Crime" => json!({"own_power":4,"opponent_big_zone":"Graveyard"}),
        "Crimestopper Sprite" => json!({"target_tapped":true,"stun":if accept{1}else{0}}),
        "Deadly Cover-Up" => {
            json!({"creatures_battlefield":0,"bob_big_zone":if accept{"Exile"}else{"Graveyard"}})
        }
        "Detective's Phoenix" => {
            json!({"host_power":if bestow{4}else{2},"host_toughness":if bestow{4}else{2},"attached":bestow})
        }
        "Extract a Confession" => {
            json!({"small_zones":vec![if accept{"Battlefield"}else{"Graveyard"};2],"big_zones":vec![if accept{"Graveyard"}else{"Battlefield"};2]})
        }
        "Ruthless Radrat" => {
            json!({"radrat_permanents":if accept{2}else{1},"two_payments_offered":if v=="surplus"{Some(true)}else{None}})
        }
        "Vitu-Ghazi Inspector" => {
            json!({"host_plus_one":if accept{1}else{0},"life":if accept{22}else{20}})
        }
        _ => unreachable!(),
    };
    let actual = match n {
        "Analyze the Pollen" => {
            json!({"hand_names":g.player(alice()).unwrap().hand.iter().map(|id|g.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()})
        }
        "Axebane Ferox" => {
            json!({"ferox_zone":zone(&g,source_pre.unwrap()),"resources_exiled":materials.iter().filter(|s|zone(&g,**s)=="Exile").count()})
        }
        "Bite Down on Crime" => {
            json!({"own_power":g.current_power(wid),"opponent_big_zone":zone(&g,opponents[0].1)})
        }
        "Crimestopper Sprite" => {
            json!({"target_tapped":g.is_tapped(current(&g,opponents[0].0)),"stun":named(&g,current(&g,opponents[0].0),"stun")})
        }
        "Deadly Cover-Up" => {
            json!({"creatures_battlefield":g.battlefield.iter().filter(|id|g.current_has_card_type(**id,ironsmith::CardType::Creature)).count(),"bob_big_zone":zone(&g,opponents[0].1)})
        }
        "Detective's Phoenix" => {
            json!({"host_power":g.current_power(wid),"host_toughness":g.current_toughness(wid),"attached":g.object(sid).unwrap().attached_to==Some(ironsmith::object::AttachmentTarget::Object(wid))})
        }
        "Extract a Confession" => {
            json!({"small_zones":opponents.iter().map(|(small,_)|zone(&g,*small)).collect::<Vec<_>>(),"big_zones":opponents.iter().map(|(_,big)|zone(&g,*big)).collect::<Vec<_>>()})
        }
        "Ruthless Radrat" => {
            json!({"radrat_permanents":g.battlefield.iter().filter(|id|g.object(**id).unwrap().name=="Ruthless Radrat").count(),"two_payments_offered":if v=="surplus"{Some(dm.optional_repeat_capacity.is_some_and(|(repeat,max)|repeat&&max.is_none_or(|m|m>=2)))}else{None}})
        }
        "Vitu-Ghazi Inspector" => {
            json!({"host_plus_one":named(&g,wid,"+1/+1"),"life":g.player(alice()).unwrap().life})
        }
        _ => unreachable!(),
    };
    Ok((
        json!({"cost":expected_cost,"error":null,"outcome":expected}),
        json!({"cost":actual_cost,"error":error,"outcome":actual}),
    ))
}
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn generate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p = root.join("reports/runtime-audit");
    let input = p.join("extended-optional-exile-inputs.json");
    let data: Value = serde_json::from_slice(&std::fs::read(&input).unwrap()).unwrap();
    let mut defs = HashMap::new();
    let mut artifacts = vec![];
    let mut rows = vec![];
    for (n, v) in data["cards"].as_object().unwrap() {
        let b = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            v["parse_name"].as_str().unwrap_or(n),
        );
        match ironsmith_registry::compile_builder_to_artifact(
            b,
            v["parse_input"].as_str().unwrap(),
            false,
        ) {
            Ok((a, d)) => {
                artifacts.push(json!({"card":n,"artifact_checksum":a.payload_checksum,"definition":a.payload.definition}));
                defs.insert(n.clone(), (d, a.payload_checksum));
            }
            Err(e) => {
                rows.push(
                    json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}}),
                );
            }
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("OPTIONAL_EXILE_CASE {c}");
        let mut dm = Dm {
            actor: alice(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: false,
            x: 0,
            plot: false,
            option_text: String::new(),
            optional_repeat_capacity: None,
        };
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&defs, c, &mut dm)));
        let (status, expected, actual) = match result {
            Ok(Ok((e, a))) => (
                if e == a {
                    "expected_result_observed"
                } else if !a["announcement_error"].is_null() || !a["resolution_error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Ok(Err(e)) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
            Err(_) => (
                "panicked",
                Value::Null,
                json!({"error":"fixture or runtime panic; inspect isolated row trace"}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs.get(n).map(|x|&x.1),"execution_trace":dm.trace}));
    }
    let report = json!({"scope":"Nine exact optional, ward and bestow Exile paths. Real paid discard generates graveyard evidence, actual paid sources and opponents, normal announcement and priority resolution. Insufficient, decline, exact and surplus evidence; actual graveyard bestow also included. No unavailable cost or action is forced.","limitations":"Initial hand/library/mana are fixtures. Graveyard materials come from normally paid One with Nothing; battlefield sources and targets are normally paid canonical spells. Insufficient optional costs are declined, and do not certify their proposal validation. Detective bestow absence is never forced. Every claimed result is explicitly scoped to the fields compared.","provenance":{"binary":std::env::current_exe().unwrap(),"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_extended_optional_exile_reproductions.rs")),"input_sha256":hash(&input),"seed":SEED,"unique_card_ids":true,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("extended-optional-exile-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("extended-optional-exile-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical Escape additional-cost expected-result reporter"]
fn report_extended_optional_exile() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}

//! Canonical paid rest-tag consumers with independent exact zone/order expectations.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, NumberContext, OrderContext, SelectObjectsContext,
    SelectOptionsContext, TargetsContext, TextInputContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with,
};
use ironsmith::game_loop::{apply_attacker_declarations_with_dm, apply_blocker_declarations};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, CounterType, GameState, ObjectId, Phase, PlayerId, Step, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
const SEED: u64 = 0x49524f4e534d4954;
fn alice() -> PlayerId {
    PlayerId(0)
}
struct Dm {
    card: String,
    targets: Vec<Target>,
    stage: String,
    trace: Vec<Value>,
    chosen: Vec<ObjectId>,
    reverse: bool,
    x: u32,
    plot: bool,
}
impl DecisionMaker for Dm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace.push(json!({"stage":self.stage,"choice":"boolean","context":format!("{c:?}"),"selected":if self.stage=="paid_primary_spell"{false}else{self.plot}}));
        if self.stage == "paid_primary_spell" {
            false
        } else {
            self.plot
        }
    }
    fn decide_targets(&mut self, _: &GameState, c: &TargetsContext) -> Vec<Target> {
        let mut selected = vec![];
        for r in &c.requirements {
            let choices = self
                .targets
                .iter()
                .copied()
                .filter(|t| r.legal_targets.contains(t) && !selected.contains(t))
                .take(r.max_targets.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                choices.len() >= r.min_targets,
                "fixture intended targets unavailable"
            );
            selected.extend(choices);
        }
        self.trace.push(json!({"stage":self.stage,"choice":"targets","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = if self.card == "Plunge into Darkness"
            && c.options.iter().any(|o| {
                o.description
                    .to_lowercase()
                    .contains("pay any amount of life")
            }) {
            vec![
                c.options
                    .iter()
                    .find(|o| {
                        o.legal
                            && o.description
                                .to_lowercase()
                                .contains("pay any amount of life")
                    })
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
        self.trace.push(json!({"stage":self.stage,"choice":"number","context":format!("{c:?}"),"selected":self.x}));
        self.x
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = if self.card == "Glorious Purpose"
            && c.description.to_lowercase().contains("discard")
        {
            c.candidates
                .iter()
                .filter(|o| o.legal && o.name == "Forest")
                .take(c.min)
                .map(|o| o.id)
                .collect::<Vec<_>>()
        } else if self.card == "Glorious Purpose" {
            c.candidates
                .iter()
                .filter(|o| o.legal && o.name == "Ornithopter" && self.plot)
                .take(c.max.unwrap_or(1))
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
        if selected.len() < c.min.min(c.candidates.iter().filter(|o| o.legal).count()) {
            self.trace.push(json!({"stage":self.stage,"choice":"unexpected_required_object_candidates","context":format!("{c:?}"),"intended":self.chosen.iter().map(|id|id.0).collect::<Vec<_>>() }));
        }
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
    fn decide_text(&mut self, _: &GameState, c: &TextInputContext) -> String {
        self.trace.push(json!({"stage":self.stage,"choice":"text","context":format!("{c:?}"),"selected":"Grizzly Bears"}));
        "Grizzly Bears".into()
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
    for s in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(s, 12);
    }
    g
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or(format!("fixture source {name} absent"))
}
fn count(g: &GameState, name: &str, z: Zone) -> usize {
    g.objects_in_deterministic_order()
        .into_iter()
        .filter(|o| o.name == name && o.zone == z)
        .count()
}
fn announce(
    g: &mut GameState,
    action: LegalAction,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    g.turn.priority_player = Some(alice());
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
    g.turn.priority_player = Some(alice());
    let id = g.create_object_from_definition(d, alice(), Zone::Hand);
    let a = compute_legal_actions(g, alice()).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("fixture source cast absent")?;
    let before = g.player(alice()).unwrap().mana_pool.total();
    announce(g, a, q, dm)?;
    let paid = before - g.player(alice()).unwrap().mana_pool.total();
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

fn normal_cast(
    g: &mut GameState,
    defs: &HashMap<String, (CardDefinition, String)>,
    n: &str,
    cost: u32,
    q: &mut TriggerQueue,
    dm: &mut Dm,
) -> Result<(), String> {
    dm.stage = format!("paid_cast_{n}");
    cast_announce(g, &defs[n].0, cost, q, dm)?;
    dm.stage = format!("resolve_{n}");
    finish(g, q, dm)
}
fn library_names(g: &GameState) -> Vec<String> {
    g.player(alice())
        .unwrap()
        .library
        .iter()
        .rev()
        .map(|id| g.object(*id).unwrap().name.to_string())
        .collect()
}
fn run(
    defs: &HashMap<String, (CardDefinition, String)>,
    c: &Value,
    dm: &mut Dm,
) -> Result<(Value, Value), String> {
    let n = c["card"].as_str().unwrap();
    let actual_name = defs[n].0.name();
    let mut g = game();
    let mut q = TriggerQueue::new();
    if n == "Archfiend of Depravity" {
        let total = c["creatures"].as_u64().unwrap() as usize;
        let keep = c["keep"].as_u64().unwrap() as usize;
        let own = c["own_end"].as_bool().unwrap();
        let mut resources = vec![];
        for _ in 0..total {
            resources.push(g.create_object_from_definition(
                &defs["Grizzly Bears"].0,
                PlayerId(1),
                Zone::Battlefield,
            ));
        }
        let cara = g.create_object_from_definition(
            &defs["Grizzly Bears"].0,
            PlayerId(2),
            Zone::Battlefield,
        );
        dm.chosen = resources.iter().take(keep).copied().collect();
        normal_cast(&mut g, defs, n, 5, &mut q, dm)?;
        if !own {
            g.next_turn();
            ironsmith::turn::execute_untap_step(&mut g);
            ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        }
        for _ in 0..3 {
            ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        }
        assert_eq!(g.turn.step, Some(Step::End));
        let event =
            ironsmith::triggers::generate_step_trigger_events(&g).ok_or("end event absent")?;
        for trigger in ironsmith::triggers::check_triggers(&g, &event) {
            q.add(trigger);
        }
        dm.stage = "actual_end_step_trigger".into();
        dm.trace
            .push(json!({"stage":dm.stage,"active":g.turn.active_player.0}));
        let error = finish(&mut g, &mut q, dm).err();
        let actual = resources
            .iter()
            .map(|id| g.object(*id).is_some_and(|o| o.zone == Zone::Battlefield))
            .collect::<Vec<_>>();
        return Ok((
            json!({"error":null,"bob_survivors":(0..total).map(|i|own||i<keep).collect::<Vec<_>>(),"cara_unaffected":true,"source_battlefield":1}),
            json!({"error":error,"bob_survivors":actual,"cara_unaffected":g.object(cara).is_some_and(|o|o.zone==Zone::Battlefield),"source_battlefield":count(&g,actual_name,Zone::Battlefield)}),
        ));
    }
    if n == "Glorious Purpose" {
        let times = c["connives"].as_u64().unwrap() as usize;
        let extra = c["extra"].as_u64().unwrap() as usize;
        for i in (0..extra).rev() {
            let d = if i == 0 {
                defs["Ornithopter"].0.clone()
            } else {
                CardDefinitionBuilder::new(CardId::new(), format!("Plan remainder {i}"))
                    .card_types(vec![CardType::Sorcery])
                    .build()
            };
            g.create_object_from_definition(&d, alice(), Zone::Library);
        }
        for _ in 0..6 {
            g.create_object_from_definition(&defs["Forest"].0, alice(), Zone::Library);
        }
        normal_cast(&mut g, defs, n, 2, &mut q, dm)?;
        let source = find(&g, actual_name)?;
        let mut error = None;
        for i in 0..times {
            dm.stage = format!("actual_connive_producer_{i}");
            if let Err(e) = normal_cast(&mut g, defs, "Raffine's Informant", 2, &mut q, dm) {
                error = Some(e);
                break;
            }
            dm.trace.push(json!({"stage":"after_connive","number":i+1,"plan_counter":g.counter_count(source,CounterType::Named("plan".into())),"source_battlefield":g.object(source).is_some_and(|o|o.zone==Zone::Battlefield)}));
        }
        let triggered = times >= 6;
        let names = (0..extra)
            .map(|i| {
                if i == 0 {
                    "Ornithopter".to_string()
                } else {
                    format!("Plan remainder {i}")
                }
            })
            .collect::<Vec<_>>();
        let actual = names
            .iter()
            .map(|name| {
                [Zone::Library, Zone::Hand, Zone::Exile, Zone::Battlefield]
                    .iter()
                    .map(|z| count(&g, name, *z))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let expected = (0..extra)
            .map(|i| {
                if !triggered {
                    vec![1, 0, 0, 0]
                } else if dm.plot && i == 0 {
                    vec![0, 0, 0, 1]
                } else {
                    vec![0, 1, 0, 0]
                }
            })
            .collect::<Vec<_>>();
        return Ok((
            json!({"error":null,"remainder_library_hand_exile_bf":expected,"source_battlefield":usize::from(!triggered),"source_graveyard":usize::from(triggered),"conniver_count":times,"conniver_plus_counters":vec![1;times],"discarded_forests":times}),
            json!({"error":error,"remainder_library_hand_exile_bf":actual,"source_battlefield":count(&g,actual_name,Zone::Battlefield),"source_graveyard":count(&g,actual_name,Zone::Graveyard),"conniver_count":count(&g,"Raffine's Informant",Zone::Battlefield),"conniver_plus_counters":g.battlefield.iter().filter(|id|g.object(**id).is_some_and(|o|o.name=="Raffine's Informant")).map(|id|g.counter_count(*id,CounterType::PlusOnePlusOne)).collect::<Vec<_>>(),"discarded_forests":count(&g,"Forest",Zone::Graveyard)}),
        ));
    }
    if n == "Vault 13: Dweller's Journey" {
        let total = c["target_count"].as_u64().unwrap() as usize;
        let mut resources = vec![];
        for i in 0..3 {
            for _ in 0..8 {
                g.create_object_from_definition(&defs["Forest"].0, PlayerId(i), Zone::Library);
            }
            let id = g.create_object_from_definition(
                &defs["Grizzly Bears"].0,
                PlayerId(i),
                Zone::Battlefield,
            );
            resources.push(g.object(id).unwrap().stable_id);
            if (i as usize) < total {
                dm.targets.push(Target::Object(id));
            }
        }
        let mut error = normal_cast(&mut g, defs, n, 4, &mut q, dm).err();
        let after_i = resources
            .iter()
            .map(|s| {
                g.find_object_by_stable_id(*s)
                    .and_then(|id| g.object(id))
                    .map(|o| format!("{:?}", o.zone))
            })
            .collect::<Vec<_>>();
        let expected_i = (0..3)
            .map(|i| if i < total { "Exile" } else { "Battlefield" })
            .collect::<Vec<_>>();
        dm.trace
            .push(json!({"stage":"actual_chapter_I","states":after_i,"error":error}));
        if error.is_some()
            || after_i
                != expected_i
                    .iter()
                    .map(|s| Some(s.to_string()))
                    .collect::<Vec<_>>()
        {
            return Ok((
                json!({"error":null,"stage":"chapter_I","zones":expected_i}),
                json!({"error":error,"stage":"chapter_I","zones":after_i}),
            ));
        }
        dm.targets.clear();
        for chapter in [2, 3] {
            for _ in 0..3 {
                g.next_turn();
                ironsmith::turn::execute_untap_step(&mut g);
            }
            assert_eq!(g.turn.active_player, alice());
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            ironsmith::turn::execute_draw_step_with(&mut g, dm).unwrap();
            ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
            assert_eq!(g.turn.phase, Phase::FirstMain);
            dm.chosen = resources
                .iter()
                .take(2.min(total))
                .filter_map(|s| g.find_object_by_stable_id(*s))
                .collect();
            dm.stage = format!("actual_precombat_main_chapter_{chapter}");
            ironsmith::game_loop::add_saga_lore_counters_with_dm(&mut g, &mut q, dm).unwrap();
            error = finish(&mut g, &mut q, dm).err();
            if error.is_some() {
                break;
            }
        }
        let zones = resources
            .iter()
            .map(|s| {
                g.find_object_by_stable_id(*s)
                    .and_then(|id| g.object(id))
                    .map(|o| format!("{:?}", o.zone))
            })
            .collect::<Vec<_>>();
        let expected = (0..3)
            .map(|i| {
                if i < total && i >= 2 {
                    "Library"
                } else {
                    "Battlefield"
                }
            })
            .collect::<Vec<_>>();
        return Ok((
            json!({"error":null,"stage":"chapter_III","zones":expected,"saga_graveyard":1,"alice_life":22}),
            json!({"error":error,"stage":"chapter_III","zones":zones,"saga_graveyard":count(&g,actual_name,Zone::Graveyard),"alice_life":g.player(alice()).unwrap().life}),
        ));
    }
    let getaway = n == "Getaway Barrel";
    let size = c["library"].as_u64().unwrap() as usize;
    let eligible = if getaway {
        c["creature"].as_bool().unwrap()
    } else {
        size > 0
    };
    let mut watched = vec![];
    for i in (0..size).rev() {
        let d = if i == 0 && eligible {
            defs["Grizzly Bears"].0.clone()
        } else {
            CardDefinitionBuilder::new(CardId::new(), format!("Event library {i:02}"))
                .card_types(vec![CardType::Sorcery])
                .build()
        };
        let id = g.create_object_from_definition(&d, alice(), Zone::Library);
        watched.push((g.object(id).unwrap().stable_id, d.name().to_string(), id));
    }
    watched.reverse();
    dm.chosen = if dm.plot && !getaway {
        watched.first().map(|x| vec![x.2]).unwrap_or_default()
    } else {
        vec![]
    };
    normal_cast(
        &mut g,
        defs,
        n,
        c["cost"].as_u64().unwrap() as u32,
        &mut q,
        dm,
    )?;
    let source = find(&g, actual_name)?;
    let error;
    if getaway {
        dm.targets = vec![Target::Object(source)];
        error = normal_cast(&mut g, defs, "Disenchant", 2, &mut q, dm).err();
    } else {
        g.remove_summoning_sickness(source);
        g.turn.phase = Phase::Combat;
        g.turn.step = Some(Step::DeclareAttackers);
        let mut combat = CombatState::default();
        dm.stage = "actual_attack".into();
        apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(PlayerId(1)),
            }],
            dm,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        finish(&mut g, &mut q, dm)?;
        g.turn.step = Some(Step::DeclareBlockers);
        apply_blocker_declarations(&mut g, &mut combat, &mut q, &[], PlayerId(1))
            .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        g.turn.step = Some(Step::CombatDamage);
        let events = ironsmith::game_loop::try_execute_combat_damage_step_with_dm(
            &mut g, &combat, false, dm,
        )
        .map_err(|e| e.to_string())?;
        dm.trace.push(json!({"stage":"actual_combat_damage","events":format!("{events:?}"),"bob_life":g.player(PlayerId(1)).unwrap().life}));
        ironsmith::game_loop::queue_combat_damage_triggers(&mut g, &events, &mut q);
        dm.stage = "actual_damage_trigger".into();
        error = finish(&mut g, &mut q, dm).err();
    }
    let looked = (if getaway { 13 } else { 6 }).min(size);
    let put = eligible && (getaway || dm.plot);
    let zones = watched
        .iter()
        .map(|(s, _, _)| {
            g.find_object_by_stable_id(*s)
                .and_then(|id| g.object(id))
                .map(|o| format!("{:?}", o.zone))
        })
        .collect::<Vec<_>>();
    let expected_zones = (0..size)
        .map(|i| {
            if i == 0 && put {
                "Battlefield"
            } else {
                "Library"
            }
        })
        .collect::<Vec<_>>();
    let mut expected_library = watched
        .iter()
        .skip(looked)
        .map(|x| x.1.clone())
        .chain(
            watched
                .iter()
                .take(looked)
                .enumerate()
                .filter(|(i, _)| !put || *i != 0)
                .map(|(_, x)| x.1.clone()),
        )
        .collect::<Vec<_>>();
    let mut actual_library = library_names(&g);
    let untouched = size.saturating_sub(looked);
    expected_library[untouched..].sort();
    if actual_library.len() >= untouched {
        actual_library[untouched..].sort();
    }
    Ok((
        json!({"error":null,"zones":expected_zones,"library_top_then_sorted_random_bottom":expected_library,"source_battlefield":usize::from(!getaway),"source_graveyard":usize::from(getaway),"bob_life":if getaway{20}else{14}}),
        json!({"error":error,"zones":zones,"library_top_then_sorted_random_bottom":actual_library,"source_battlefield":count(&g,actual_name,Zone::Battlefield),"source_graveyard":count(&g,actual_name,Zone::Graveyard),"bob_life":g.player(PlayerId(1)).unwrap().life}),
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
    let input = p.join("rest-event-inputs.json");
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
            Err(e) => rows
                .push(json!({"card":n,"status":"compile_failed","actual":{"error":e.to_string()}})),
        }
    }
    for c in data["cases"].as_array().unwrap() {
        let n = c["card"].as_str().unwrap();
        eprintln!("REST_EVENT {c}");
        let mut dm = Dm {
            card: n.into(),
            targets: vec![],
            stage: "fixture".into(),
            trace: vec![],
            chosen: vec![],
            reverse: c["reverse"].as_bool().unwrap_or(false),
            x: c["life"].as_u64().unwrap_or(0) as u32,
            plot: c["accept"].as_bool().unwrap_or(false),
        };
        let result = run(&defs, c, &mut dm);
        let (status, expected, actual) = match result {
            Ok((e, a)) => (
                if a == e {
                    "expected_result_observed"
                } else if !a["error"].is_null() {
                    "resolution_failed"
                } else {
                    "semantic_mismatch"
                },
                e,
                a,
            ),
            Err(e) => (
                "execution_or_fixture_error",
                Value::Null,
                json!({"error":e}),
            ),
        };
        rows.push(json!({"card":n,"scenario":c,"status":status,"expected":expected,"actual":actual,"artifact_checksum":defs[n].1,"execution_trace":dm.trace}));
    }
    let binary = std::env::current_exe().unwrap();
    let files = [
        "crates/ironsmith-engine/src/effects/zones/move_to_zone.rs",
        "crates/ironsmith-engine/src/effects/cards/look_at_top.rs",
        "crates/ironsmith-engine/src/effects/composition/choose_objects.rs",
    ];
    let report = json!({"scope":"Remaining canonical rest-tag sources via paid casts and real end step, paid Disenchant death, unblocked combat damage, six paid connive ETBs, and actual saga lore turn-based actions.","limitations":"Mana and inert resources/library witnesses seeded. Combat and appropriate main-phase windows are positioned explicitly; source summoning sickness cleared for combat. Normal priority/SBA resolver used. Random remainder order checked as a multiset below exact untouched prefix. Earlier chapter/choice gates recorded distinctly. No injected effect context or engine edits.","provenance":{"binary":binary,"binary_sha256":std::env::var("AUDIT_BINARY_SHA256").unwrap(),"source_sha256":hash(&root.join("crates/ironsmith-tools/tests/runtime_rest_event_reproductions.rs")),"runtime_source_hashes":files.iter().map(|f|json!({"path":f,"sha256":hash(&root.join(f))})).collect::<Vec<_>>(),"input_sha256":hash(&input),"unique_card_ids":true,"seed":SEED,"thread_stack_bytes":67108864,"compiled_via":"ironsmith_registry::compile_builder_to_artifact"},"rows":rows});
    std::fs::write(
        p.join("rest-event-artifacts.json"),
        serde_json::to_string_pretty(&artifacts).unwrap(),
    )
    .unwrap();
    std::fs::write(
        p.join("rest-event-reproductions.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "canonical rest-event expected-result reporter"]
fn report_rest_events() {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(generate)
        .unwrap()
        .join()
        .unwrap();
}

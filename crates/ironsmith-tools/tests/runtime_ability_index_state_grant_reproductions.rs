//! Paid conditional-index transitions through counters, exile history, tap and board state.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
fn hash(p: &std::path::Path) -> String {
    Sha256::digest(std::fs::read(p).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Choices {
    accept: bool,
    target: Option<Target>,
    trace: Vec<Value>,
    resources: Vec<ObjectId>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool {
        false
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"decision":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let a = if let Some(o) = c
            .options
            .iter()
            .find(|o| o.legal && o.description == "Untap")
        {
            vec![o.index]
        } else {
            SelectFirstDecisionMaker.decide_options(g, c)
        };
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
        a
    }
    fn decide_objects(
        &mut self,
        g: &GameState,
        c: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let preferred: Vec<_> = self
            .resources
            .iter()
            .copied()
            .filter(|id| c.candidates.iter().any(|o| o.id == *id && o.legal))
            .collect();
        let a = if preferred.len() >= c.min && !preferred.is_empty() {
            preferred
                .into_iter()
                .take(c.max.unwrap_or(self.resources.len()))
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.trace.push(
            json!({"decision":"objects","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
    fn decide_targets(&mut self, _g: &GameState, c: &TargetsContext) -> Vec<Target> {
        let a = c
            .requirements
            .iter()
            .filter_map(|r| {
                self.target
                    .filter(|t| r.legal_targets.contains(t))
                    .or_else(|| r.legal_targets.first().copied())
            })
            .collect::<Vec<_>>();
        self.trace.push(
            json!({"decision":"targets","context":format!("{c:?}"),"answer":format!("{a:?}")}),
        );
        a
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
    eprintln!("AUDIT_STAGE cast {}", def.name());
    g.turn.priority_player = Some(actor);
    let id = g.create_object_from_definition(def, actor, Zone::Hand);
    let action = compute_legal_actions(g, actor).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or_else(|| format!("{} normal cast unavailable", def.name()))?;
    let before = g.player(actor).unwrap().mana_pool.total();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace.push(json!({"stage":"legal_cast","card":def.name(),"actor":actor.index(),"action":format!("{action:?}")}));
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_cast.is_none()
            && g.stack.iter().any(|e| {
                !e.is_ability && g.object(e.object_id).is_some_and(|o| o.name == def.name())
            })
        {
            let paid = before - g.player(actor).unwrap().mana_pool.total();
            dm.trace.push(json!({"stage":"cast_complete","card":def.name(),"paid":paid,"stack":format!("{:?}",g.stack)}));
            return Ok(paid);
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("cast stalled:{progress:?}"));
        };
        progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("cast announcement bound".into())
}
fn resolve_one(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    state.reset_for_new_priority_window(g);
    // The final pass invokes the production resolver, which retains the trigger
    // queue and therefore produces dedicated ETB events as well as zone changes.
    for _ in 0..g.players_in_game() {
        let progress = apply_priority_response_with_dm(
            g,
            q,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            dm,
        )
        .map_err(|e| e.to_string())?;
        if let GameProgress::NeedsDecisionCtx(ctx) = &progress {
            if !matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Err(format!(
                    "priority resolution requires unhandled decision:{ctx:?}"
                ));
            }
        }
    }
    Ok(())
}
fn drain(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    drain_pending_trigger_events(g, q);
    put_triggers_on_stack_with_dm(g, q, dm).map_err(|e| e.to_string())
}
fn resolve_all(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..24 {
        drain(g, q, dm)?;
        if g.stack.is_empty() {
            return Ok(());
        }
        resolve_one(g, q, dm)?;
    }
    Err("bounded resolution did not empty stack".into())
}
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
    eprintln!("AUDIT_STAGE activate {action:?}");
    let old = g.stack.len();
    let mut state = PriorityLoopState::new(g.players_in_game());
    dm.trace
        .push(json!({"stage":"activation_action","action":format!("{action:?}")}));
    let mut p = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if state.pending_activation.is_none() && g.stack.len() > old {
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = p else {
            return Err(format!("activation stalled:{p:?}"));
        };
        p = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("activation bound".into())
}

fn mana(g: &mut GameState) {
    for p in [PlayerId(0), PlayerId(1)] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            g.player_mut(p).unwrap().mana_pool.add(symbol, 12);
        }
    }
}
fn find(g: &GameState, name: &str) -> Result<ObjectId, String> {
    g.battlefield
        .iter()
        .copied()
        .find(|id| g.object(*id).is_some_and(|o| o.name == name))
        .ok_or_else(|| format!("{name} absent from battlefield"))
}

fn advance_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    for e in ironsmith::turn::execute_draw_step_with(g, dm).unwrap() {
        for t in ironsmith::triggers::check_triggers(g, &e) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(PlayerId(0));
    mana(g);
    Ok(())
}
fn announce_mana(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    let mut state = PriorityLoopState::new(g.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        g,
        q,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..32 {
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            if matches!(
                ctx,
                ironsmith::decisions::context::DecisionContext::Priority(_)
            ) {
                return Ok(());
            }
            progress = apply_decision_context_with_dm(g, q, &mut state, &ctx, dm)
                .map_err(|e| e.to_string())?;
        } else {
            return Ok(());
        }
    }
    Err("mana announcement bound".into())
}

fn activation(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
) -> Result<Value, String> {
    g.turn.priority_player = Some(PlayerId(0));
    let a = compute_legal_actions(g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source));
    let before = g.player(PlayerId(0)).unwrap().mana_pool.total() as i64;
    let mut error = None;
    let mut resolution_error = None;
    if let Some(action) = a.clone() {
        dm.trace.push(json!({"stage":"advertised_source_action","action":format!("{action:?}"),"abilities":format!("{:?}",g.current_abilities(source))}));
        error = announce(g, q, dm, action).err();
        if error.is_none() {
            resolution_error = resolve_all(g, q, dm).err();
        }
    }
    Ok(
        json!({"offered":a.is_some(),"announcement_error":error,"resolution_error":resolution_error,"mana_paid":before-g.player(PlayerId(0)).unwrap().mana_pool.total() as i64,"remaining_stack":g.stack.len()}),
    )
}
fn paid(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    source: ObjectId,
    cost: i64,
) -> Result<Value, String> {
    let a = activation(g, q, dm, source)?;
    if a != json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":cost,"remaining_stack":0})
    {
        return Err(format!("prerequisite activation did not complete: {a}"));
    }
    Ok(a)
}
fn stats(g: &GameState, source: ObjectId) -> Value {
    use ironsmith::static_abilities::StaticAbilityId as K;
    json!({"power":g.calculated_power(source),"toughness":g.calculated_toughness(source),"plus_counters":g.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),"trample":g.object_has_static_ability_id(source,K::Trample),"flying":g.object_has_static_ability_id(source,K::Flying),"vigilance":g.object_has_static_ability_id(source,K::Vigilance),"hexproof":g.object_has_static_ability_id(source,K::Hexproof),"haste":g.object_has_static_ability_id(source,K::Haste)})
}
fn run(
    def: &CardDefinition,
    twiddle: &CardDefinition,
    jace: &CardDefinition,
    mode: usize,
) -> Result<Value, String> {
    use ironsmith::static_abilities::StaticAbilityId as K;
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let witness_def = CardDefinitionBuilder::new(CardId::new(), "Neutral legendary recipient")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![ironsmith::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 6))
        .build();
    let witness = g.create_object_from_definition(&witness_def, PlayerId(0), Zone::Battlefield);
    let ad = CardDefinitionBuilder::new(CardId::new(), "Neutral artifact cost resource")
        .card_types(vec![CardType::Artifact])
        .build();
    let resources: Vec<_> = (0..3)
        .map(|_| g.create_object_from_definition(&ad, PlayerId(0), Zone::Battlefield))
        .collect();
    for player in [PlayerId(0), PlayerId(1)] {
        for _ in 0..12 {
            g.create_object_from_definition(
                &CardDefinitionBuilder::new(CardId::new(), "Neutral library card")
                    .card_types(vec![CardType::Artifact])
                    .build(),
                player,
                Zone::Library,
            );
        }
    }
    let mut q = TriggerQueue::new();
    let mut dm = Choices {
        accept: false,
        target: None,
        trace: vec![],
        resources: resources.clone(),
    };
    let cost = def.card.mana_cost.as_ref().unwrap().mana_value();
    let source_paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
    if source_paid != cost {
        return Err("source payment differs from printed cost".into());
    }
    resolve_all(&mut g, &mut q, &mut dm)?;
    let source = find(&g, def.name())?;
    let mut evidence = json!({"mode":mode,"source_cast_paid":source_paid});
    let expected;
    let actual;
    if def.name() == "Keen-Eyed Curator" {
        let before_count = [0, 3, 4][mode];
        let types = [
            CardType::Creature,
            CardType::Artifact,
            CardType::Enchantment,
            CardType::Sorcery,
            CardType::Land,
        ];
        let mut gy = vec![];
        for (i, kind) in types.into_iter().enumerate() {
            let mut b =
                CardDefinitionBuilder::new(CardId::new(), format!("Exile type witness {i}"))
                    .card_types(vec![kind]);
            if kind == CardType::Creature {
                b = b.power_toughness(PowerToughness::fixed(2, 2));
            }
            gy.push(g.create_object_from_definition(&b.build(), PlayerId(0), Zone::Graveyard));
        }
        for id in gy.iter().take(before_count) {
            dm.target = Some(Target::Object(*id));
            paid(&mut g, &mut q, &mut dm, source, 1)?;
        }
        evidence["before_stats"] = stats(&g, source);
        evidence["earlier_paid_exiles"] = json!(before_count);
        dm.target = Some(Target::Object(gy[before_count]));
        let outcome = activation(&mut g, &mut q, &mut dm, source)?;
        let after = before_count + 1;
        let exiled = g
            .objects_in_deterministic_order()
            .iter()
            .filter(|o| o.name.starts_with("Exile type witness") && o.zone == Zone::Exile)
            .count();
        expected = json!({"activation":{"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":1,"remaining_stack":0},"before_power":if before_count>=4{7}else{3},"before_trample":before_count>=4,"power":if after>=4{7}else{3},"toughness":if after>=4{7}else{3},"trample":after>=4,"exiled_types":after});
        actual = json!({"activation":outcome,"before_power":evidence["before_stats"]["power"],"before_trample":evidence["before_stats"]["trample"],"power":g.calculated_power(source),"toughness":g.calculated_toughness(source),"trample":g.object_has_static_ability_id(source,K::Trample),"exiled_types":exiled});
    } else if def.name() == "Warden of the Inner Sky" {
        let prior = [0, 2, 3][mode];
        for _ in 0..prior {
            paid(&mut g, &mut q, &mut dm, source, 0)?;
            advance_turn(&mut g, &mut q, &mut dm)?;
            advance_turn(&mut g, &mut q, &mut dm)?;
        }
        evidence["before_stats"] = stats(&g, source);
        let outcome = activation(&mut g, &mut q, &mut dm, source)?;
        let after = prior + 1;
        expected = json!({"activation":{"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":0,"remaining_stack":0},"before_counters":prior,"before_flying":prior>=3,"power":1+after,"toughness":2+after,"counters":after,"flying":after>=3,"vigilance":after>=3,"tapped_cost_artifacts":3});
        actual = json!({"activation":outcome,"before_counters":evidence["before_stats"]["plus_counters"],"before_flying":evidence["before_stats"]["flying"],"power":g.calculated_power(source),"toughness":g.calculated_toughness(source),"counters":g.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),"flying":g.object_has_static_ability_id(source,K::Flying),"vigilance":g.object_has_static_ability_id(source,K::Vigilance),"tapped_cost_artifacts":resources.iter().filter(|id|g.is_tapped(**id)).count()});
    } else if def.name() == "K-9, Mark I" {
        advance_turn(&mut g, &mut q, &mut dm)?;
        advance_turn(&mut g, &mut q, &mut dm)?;
        if g.is_summoning_sick(source) {
            return Err("K9 source not ready after actual untap".into());
        }
        if mode == 1 {
            dm.target = Some(Target::Object(witness));
            paid(&mut g, &mut q, &mut dm, source, 2)?;
            let offered = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
                .iter()
                .any(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}if *s==source));
            evidence["tapped_source_offered_activation"] = json!(offered);
            evidence["ward_while_source_tapped"] =
                json!(g.object_has_static_ability_id(witness, K::Ward));
            dm.target = Some(Target::Object(source));
            dm.accept = true;
            let paid_twiddle = cast(&mut g, twiddle, PlayerId(0), &mut q, &mut dm)?;
            if paid_twiddle != 1 {
                return Err("Twiddle payment not1".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            if g.is_tapped(source) {
                return Err("real Twiddle did not untap source".into());
            }
            dm.accept = false;
        }
        let ward_before = g.object_has_static_ability_id(witness, K::Ward);
        dm.target = Some(Target::Object(witness));
        let outcome = if mode == 2 {
            json!({"not_requested":true})
        } else {
            activation(&mut g, &mut q, &mut dm, source)?
        };
        let blocker = g.create_object_from_definition(&witness_def, PlayerId(1), Zone::Battlefield);
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let mut combat = ironsmith::combat_state::CombatState::default();
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[ironsmith::decision::AttackerDeclaration {
                creature: witness,
                target: ironsmith::combat_state::AttackTarget::Player(PlayerId(1)),
            }],
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat.clone());
        resolve_all(&mut g, &mut q, &mut dm)?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let options = ironsmith::decision::compute_legal_blockers(&g, &combat, PlayerId(1));
        let can_block = options
            .iter()
            .any(|o| o.attacker == witness && o.valid_blockers.contains(&blocker));
        let blocks = if can_block {
            vec![ironsmith::decision::BlockerDeclaration {
                blocker,
                blocking: witness,
            }]
        } else {
            vec![]
        };
        ironsmith::game_loop::apply_blocker_declarations(
            &mut g,
            &mut combat,
            &mut q,
            &blocks,
            PlayerId(1),
        )
        .map_err(|e| e.to_string())?;
        let blocked = ironsmith::combat_state::is_blocked(&combat, witness);
        g.combat = Some(combat);
        resolve_all(&mut g, &mut q, &mut dm)?;
        evidence["actual_block_options"] = json!(format!("{options:?}"));
        evidence["actual_block_declarations"] = json!(format!("{blocks:?}"));
        evidence["unblockable_marker_metadata_only"] =
            json!(g.object_has_static_ability_id(witness, K::Unblockable));
        expected = json!({"activation":if mode==2{json!({"not_requested":true})}else{json!({"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":2,"remaining_stack":0})},"recipient_ward_before":true,"recipient_ward_after":mode==2,"recipient_can_be_blocked":mode==2,"block_declared":mode==2,"source_tapped":mode!=2});
        actual = json!({"activation":outcome,"recipient_ward_before":ward_before,"recipient_ward_after":g.object_has_static_ability_id(witness,K::Ward),"recipient_can_be_blocked":can_block,"block_declared":blocked,"source_tapped":g.is_tapped(source)});
    } else {
        advance_turn(&mut g, &mut q, &mut dm)?;
        if mode == 1 {
            let jc = cast(&mut g, jace, PlayerId(1), &mut q, &mut dm)?;
            if jc != 3 {
                return Err("Jace cast payment not3".into());
            }
            resolve_all(&mut g, &mut q, &mut dm)?;
            evidence["opponent_planeswalker_paid"] = json!(jc);
        }
        advance_turn(&mut g, &mut q, &mut dm)?;
        if g.is_summoning_sick(source) {
            return Err("Syr Ginger source not ready".into());
        }
        evidence["before_stats"] = stats(&g, source);
        let outcome = activation(&mut g, &mut q, &mut dm, source)?;
        expected = json!({"activation":{"offered":true,"announcement_error":null,"resolution_error":null,"mana_paid":2,"remaining_stack":0},"before_trample":mode==1,"before_hexproof":mode==1,"before_haste":mode==1,"alice_life":23,"source_in_graveyard":true});
        actual = json!({"activation":outcome,"before_trample":evidence["before_stats"]["trample"],"before_hexproof":evidence["before_stats"]["hexproof"],"before_haste":evidence["before_stats"]["haste"],"alice_life":g.player(PlayerId(0)).unwrap().life,"source_in_graveyard":g.player(PlayerId(0)).unwrap().graveyard.iter().any(|id|g.object(*id).is_some_and(|o|o.name==def.name()))});
    }
    Ok(
        json!({"expected":expected,"actual":actual,"state_evidence":evidence,"execution_trace":dm.trace}),
    )
}
#[test]
#[ignore = "manual scoped conditional index state transition report"]
fn report_state_grant_activations() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let source = root
        .join("crates/ironsmith-tools/tests/runtime_ability_index_state_grant_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names = [
        "Keen-Eyed Curator",
        "Warden of the Inner Sky",
        "K-9, Mark I",
        "Syr Ginger, the Meal Ender",
    ];
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.into_iter().chain(["Twiddle", "Jace Beleren"]) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p["parse_name"].as_str().unwrap_or(name),
            ),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name, def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_STATIC_CARD").is_ok_and(|n| n != name) {
            continue;
        }
        let count = if name == "Keen-Eyed Curator"
            || name == "Warden of the Inner Sky"
            || name == "K-9, Mark I"
        {
            3
        } else {
            2
        };
        for mode in 0..count {
            if std::env::var("AUDIT_STATIC_MODE").is_ok_and(|n| n != mode.to_string()) {
                continue;
            }
            eprintln!("AUDIT_CASE {name} {mode}");
            let (status, out) =
                match run(&defs[name], &defs["Twiddle"], &defs["Jace Beleren"], mode) {
                    Ok(out) => {
                        let status =
                            if out["actual"]["activation"]["announcement_error"].is_string() {
                                "action_or_choice_failed"
                            } else if out["actual"]["activation"]["resolution_error"].is_string() {
                                "resolution_failed"
                            } else if out["expected"] == out["actual"] {
                                "expected_result_observed"
                            } else {
                                "semantic_mismatch"
                            };
                        (status, out)
                    }
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"expected":null,"actual":{"error":e}}),
                    ),
                };
            rows.push(json!({"card":name,"scenario":{"mode":mode},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Paid canonical source and real printed producer actions, actual turn/untap transitions as required, resource-complete advertised activation. Exact condition characteristics, costs and outcome checked; linked-face and unrelated abilities remain outside scope."}));
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    let out = std::env::var("AUDIT_STATIC_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            root.join("reports/runtime-audit/ability-index-state-grant-reproductions.json")
        });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}

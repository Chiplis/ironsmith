//! Paid source and exact mana tests for conditional static wrappers.
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
        let a = if let Some(o) = c.options.iter().find(|o|o.legal && o.description == "Untap") {vec![o.index]} else {SelectFirstDecisionMaker.decide_options(g, c)};
        self.trace
            .push(json!({"decision":"options","context":format!("{c:?}"),"answer":a}));
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
fn basic(name: &str, kind: CardType) -> CardDefinition {
    let b = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]);
    if kind == CardType::Creature {
        b.power_toughness(PowerToughness::fixed(2, 6)).build()
    } else {
        b.build()
    }
}
fn cast(
    g: &mut GameState,
    def: &CardDefinition,
    actor: PlayerId,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<u32, String> {
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
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
fn announce(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
    action: LegalAction,
) -> Result<(), String> {
    g.turn.priority_player = Some(PlayerId(0));
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
fn start_next_turn(
    g: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut Choices,
) -> Result<(), String> {
    g.next_turn();
    ironsmith::turn::execute_untap_step(g);
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    ironsmith::turn::advance_step(g).map_err(|e| e.to_string())?;
    let events = ironsmith::turn::execute_draw_step_with(g, dm);
    for event in events {
        for t in ironsmith::triggers::check_triggers(g, &event) {
            q.add(t);
        }
    }
    resolve_all(g, q, dm)?;
    ironsmith::turn::advance_phase(g).map_err(|e| e.to_string())?;
    g.turn.priority_player = Some(g.turn.active_player);
    dm.trace.push(json!({"stage":"next_turn_main","active":g.turn.active_player.index(),"turn":g.turn.turn_number,"phase":format!("{:?}",g.turn.phase)}));
    Ok(())
}
fn next_own_turn(g: &mut GameState, q: &mut TriggerQueue, dm: &mut Choices) -> Result<(), String> {
    for _ in 0..g.players_in_game() {
        start_next_turn(g, q, dm)?;
    }
    Ok(())
}

use ironsmith::Supertype;

use ironsmith::Subtype;
fn mana(g: &mut GameState) {
    for sym in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(PlayerId(0)).unwrap().mana_pool.add(sym, 12);
    }
}
fn run(
    def: &CardDefinition,
    attachment_producer: &CardDefinition,
    condition_on: bool,
    ordinal: usize,
) -> Result<Value, String> {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
    g.set_random_seed(71757432704855);
    g.turn.turn_number = 3;
    g.turn.active_player = PlayerId(0);
    g.turn.priority_player = Some(PlayerId(0));
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    mana(&mut g);
    let witness_def = CardDefinitionBuilder::new(CardId::new(), "Legal activation witness")
        .card_types(vec![CardType::Creature])
        .subtypes(if condition_on {
            if def.name() == "Drover of the Mighty" {
                vec![Subtype::Dinosaur]
            } else if def.name() == "Barrow Naughty" {
                vec![Subtype::Faerie]
            } else {
                vec![Subtype::Human, Subtype::Warrior]
            }
        } else {
            vec![Subtype::Bear]
        })
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 6))
        .build();
    let witness = g.create_object_from_definition(&witness_def, PlayerId(0), Zone::Battlefield);
    if condition_on && def.name() == "Convergence of Dominion" {
        g.set_as_commander(witness, PlayerId(0));
    }
    let library = basic("Neutral ability-index library card", CardType::Artifact);
    let neutral_land = basic("Neutral ability-index land", CardType::Land);
    for seat in 0..2 {
        let p = PlayerId(seat);
        for _ in 0..8 {
            g.create_object_from_definition(&library, p, Zone::Library);
            g.create_object_from_definition(&neutral_land, p, Zone::Library);
        }
        for _ in 0..3 {
            g.create_object_from_definition(&library, p, Zone::Hand);
        }
    }
    g.create_object_from_definition(&neutral_land, PlayerId(0), Zone::Battlefield);
    if condition_on {
        for subtype in [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ] {
            let land = CardDefinitionBuilder::new(CardId::new(), "Typed activation condition land")
                .card_types(vec![CardType::Land])
                .subtypes(vec![subtype])
                .supertypes(vec![Supertype::Basic])
                .build();
            g.create_object_from_definition(&land, PlayerId(0), Zone::Battlefield);
        }
        for _ in 0..3 {
            g.create_object_from_definition(&neutral_land, PlayerId(0), Zone::Graveyard);
        }
        for _ in 0..4 {
            g.create_object_from_definition(
                &basic("Graveyard condition instant", CardType::Instant),
                PlayerId(0),
                Zone::Graveyard,
            );
        }
    }
    if condition_on {
        if def.name() == "Reclusive Taxidermist" {
            for _ in 0..4 {
                g.create_object_from_definition(
                    &basic("Creature graveyard condition", CardType::Creature),
                    PlayerId(0),
                    Zone::Graveyard,
                );
            }
        }
        if def.name() == "Oketra the True" {
            for _ in 0..2 {
                g.create_object_from_definition(
                    &basic("Other creature condition", CardType::Creature),
                    PlayerId(0),
                    Zone::Battlefield,
                );
            }
        }
        if def.name() == "Avatar of Fury" {
            for _ in 0..7 {
                g.create_object_from_definition(&neutral_land, PlayerId(1), Zone::Battlefield);
            }
        }
        if def.name() == "Nylea, God of the Hunt" {
            let d =
                CardDefinitionBuilder::new(CardId::new(), "Established green devotion permanent")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(4, 4))
                    .mana_cost(ironsmith::mana::ManaCost::from_symbols(
                        vec![ManaSymbol::Green; 4],
                    ))
                    .build();
            g.create_object_from_definition(&d, PlayerId(0), Zone::Battlefield);
        }
    }
    if condition_on && def.name() == "Avatar of Woe" {
        for p in [PlayerId(0),PlayerId(1)] { for _ in 0..5 {g.create_object_from_definition(&basic("Prior creature card",CardType::Creature),p,Zone::Graveyard);}}
    }
    let devotion_color = match def.name() {
        "Erebos, Bleak-Hearted" | "Erebos, God of the Dead" | "Pharika, God of Affliction" => ManaSymbol::Black,
        "Heliod, God of the Sun" | "Heliod, Sun-Crowned" => ManaSymbol::White,
        "Nylea, Keen-Eyed" => ManaSymbol::Green,
        "Purphoros, Bronze-Blooded" | "Purphoros, God of the Forge" => ManaSymbol::Red,
        _ => ManaSymbol::Blue,
    };
    if condition_on {
        let established = CardDefinitionBuilder::new(CardId::new(), "Established devotion enchantment")
            .card_types(vec![CardType::Enchantment])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![devotion_color; if def.name() == "Pharika, God of Affliction" {5} else {4}]))
            .build();
        g.create_object_from_definition(&established, PlayerId(0), Zone::Battlefield);
    }
    let resource = g.create_object_from_definition(&basic("Prior creature graveyard resource", CardType::Creature), PlayerId(0), Zone::Graveyard);
    let enemy = g.create_object_from_definition(&basic("Opposing activation recipient", CardType::Creature), PlayerId(1), Zone::Battlefield);
    let mut dm = Choices {
        accept: true,
        target: Some(Target::Object(if def.name() == "Pharika, God of Affliction" {resource} else if def.name() == "Erebos, Bleak-Hearted" {enemy} else {witness})),
        trace: vec![],
    };
    let mut q = TriggerQueue::new();
    if condition_on && def.card.subtypes.contains(&Subtype::Equipment) {
        let paid = cast(&mut g, attachment_producer, PlayerId(0), &mut q, &mut dm)?;
        if paid != 4 {
            return Err("Hammer of Nazahn source payment incorrect".into());
        }
        resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("Hammer setup:{e}"))?;
        dm.trace.push(json!({"stage":"canonical_attachment_producer_established","card":attachment_producer.name(),"paid":paid}));
    }
    let paid;
    if def.card.card_types.contains(&CardType::Land) {
        let hand = g.create_object_from_definition(def, PlayerId(0), Zone::Hand);
        let action = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
            .into_iter()
            .find(|a| matches!(a,LegalAction::PlayLand{land_id}if *land_id==hand))
            .ok_or("canonical land play unavailable")?;
        let mut state = PriorityLoopState::new(g.players_in_game());
        apply_priority_response_with_dm(
            &mut g,
            &mut q,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        paid = 0;
    } else {
        paid = cast(&mut g, def, PlayerId(0), &mut q, &mut dm)?;
        let want = if matches!(def.name(), "Avatar of Fury" | "Avatar of Woe") && condition_on {
            2
        } else {
            def.card.mana_cost.as_ref().map_or(0, |m| m.mana_value())
        };
        if paid != want {
            return Err(format!("source paid{paid} expected{want}"));
        }
    }
    resolve_all(&mut g, &mut q, &mut dm).map_err(|e| format!("canonical source setup:{e}"))?;
    let source = *g
        .battlefield
        .iter()
        .find(|id| g.object(**id).is_some_and(|o| o.name == def.name()))
        .ok_or("source absent after cast")?;
    next_own_turn(&mut g, &mut q, &mut dm)?;
    if def.name() == "Prava of the Steel Legion" && !condition_on {
        start_next_turn(&mut g, &mut q, &mut dm)?;
    }
    mana(&mut g);
    if def.name() == "Arixmethes, Slumbering Isle" {
        if g.counter_count(source,ironsmith::object::CounterType::Named("slumber".into())) != 5 {return Err("Arixmethes did not actually enter with five slumber counters".into());}
        if condition_on {
            dm.target=Some(Target::Object(source));
            for remaining in (0..5).rev() {
                let paid=cast(&mut g,attachment_producer,PlayerId(0),&mut q,&mut dm)?;
                if paid != 1 {return Err("Twiddle spell payment incorrect".into());}
                resolve_all(&mut g,&mut q,&mut dm)?;
                if g.counter_count(source,ironsmith::object::CounterType::Named("slumber".into())) != remaining {return Err(format!("actual cast trigger did not remove slumber to {remaining}"));}
            }
            dm.target=Some(Target::Object(witness));
            if g.is_tapped(source) {return Err("last real Twiddle did not untap source".into());}
        }
    }
    let mut evidence = json!({"condition_on":condition_on,"source":source.0,"source_cast_paid":paid,"source_definition_abilities":format!("{:?}",def.abilities)});
    evidence["source_is_creature_before_activation"] = json!(g.calculated_card_types(source).contains(&CardType::Creature));
    evidence["devotion"] = json!({"white":g.devotion_to_color(PlayerId(0),ironsmith::color::Color::White),"blue":g.devotion_to_color(PlayerId(0),ironsmith::color::Color::Blue),"black":g.devotion_to_color(PlayerId(0),ironsmith::color::Color::Black),"red":g.devotion_to_color(PlayerId(0),ironsmith::color::Color::Red),"green":g.devotion_to_color(PlayerId(0),ironsmith::color::Color::Green)});

    if condition_on && def.card.subtypes.contains(&Subtype::Equipment) {
        if g.object(source).unwrap().attached_to
            != Some(ironsmith::object::AttachmentTarget::Object(witness))
        {
            return Err(format!(
                "canonical Hammer ETB did not attach source Equipment:{:?}",
                g.object(source).unwrap().attached_to
            ));
        }
        evidence["actual_canonical_attachment"] = json!(true);
    }
    if condition_on && matches!(def.name(), "Adanto Vanguard" | "Basandra, Battle Seraph") {
        ironsmith::turn::advance_phase(&mut g).map_err(|e| e.to_string())?;
        ironsmith::turn::advance_step(&mut g).map_err(|e| e.to_string())?;
        let mut combat = CombatState::default();
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut g,
            &mut combat,
            &mut q,
            &[AttackerDeclaration {
                creature: source,
                target: AttackTarget::Player(PlayerId(1)),
            }],
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        g.combat = Some(combat);
        resolve_all(&mut g, &mut q, &mut dm)?;
        mana(&mut g);
        evidence["actual_source_attack"] = json!(true);
    }
    g.turn.priority_player = Some(PlayerId(0));
    let actions: Vec<_> = compute_legal_actions(&g, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .filter(|a| matches!(a,LegalAction::ActivateAbility{source:s,..}|LegalAction::ActivateManaAbility{source:s,..}if *s==source))
        .collect();
    evidence["offered_source_actions"] = json!(format!("{actions:?}"));
    evidence["current_abilities"] = json!(format!("{:?}", g.current_abilities(source)));
    evidence["source_power"] = json!(g.calculated_power(source));
    evidence["source_toughness"] = json!(g.calculated_toughness(source));
    evidence["source_attached_to"] = json!(format!("{:?}", g.object(source).unwrap().attached_to));
    evidence["witness_power"] = json!(g.calculated_power(witness));
    evidence["phase"] = json!(format!("{:?}", g.turn));
    evidence["graveyard_count"] = json!(g.player(PlayerId(0)).unwrap().graveyard.len());
    evidence["controls_commander"] = json!(g.player_controls_a_commander(PlayerId(0)));
    evidence["slumber_counters_before"] = json!(g.counter_count(source,ironsmith::object::CounterType::Named("slumber".into())));
    evidence["depletion_counters_before"] = json!(g.counter_count(source,ironsmith::object::CounterType::Depletion));
    let action = actions.get(ordinal).cloned();
    let offered = action.is_some();
    let mut announcement_error = None;
    let mut resolution_error = None;
    if let Some(action) = action {
        if let LegalAction::ActivateAbility { ability_index, .. }
        | LegalAction::ActivateManaAbility { ability_index, .. } = action
        {
            evidence["selected_index"] = json!(ability_index);
            evidence["dispatcher_current_ability"] =
                json!(format!("{:?}", g.current_ability(source, ability_index)));
        }
        let before = g.player(PlayerId(0)).unwrap().mana_pool.total();
        let life = g.player(PlayerId(0)).unwrap().life;
        if matches!(action, LegalAction::ActivateManaAbility { .. }) {
            let mut state = PriorityLoopState::new(g.players_in_game());
            let result = (|| -> Result<(), String> {
                let mut progress = apply_priority_response_with_dm(
                    &mut g,
                    &mut q,
                    &mut state,
                    &PriorityResponse::PriorityAction(action.clone()),
                    &mut dm,
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
                        progress = apply_decision_context_with_dm(
                            &mut g, &mut q, &mut state, &ctx, &mut dm,
                        )
                        .map_err(|e| e.to_string())?;
                    } else {
                        return Ok(());
                    }
                }
                Err("mana action decision bound".into())
            })();
            announcement_error = result.err();
            evidence["mana_action_total_added"] =
                json!(g.player(PlayerId(0)).unwrap().mana_pool.total() as i64 - before as i64);
            evidence["mana_action_source_tapped"] = json!(g.is_tapped(source));
        } else {
            announcement_error = announce(&mut g, &mut q, &mut dm, action).err();
        }
        evidence["activation_mana_paid"] =
            json!(before as i64 - g.player(PlayerId(0)).unwrap().mana_pool.total() as i64);
        evidence["activation_life_paid"] = json!(life - g.player(PlayerId(0)).unwrap().life);
        if announcement_error.is_none() {
            resolution_error = resolve_all(&mut g, &mut q, &mut dm).err();
        }
    }
    let mut out = json!({"expected":{"activation_offered":true,"announcement_error":null,"resolution_error":null,"remaining_stack":0},"actual":{"activation_offered":offered,"announcement_error":announcement_error,"resolution_error":resolution_error,"remaining_stack":g.stack.len()},"state_evidence":evidence,"execution_trace":dm.trace});
    if out["state_evidence"]
        .get("mana_action_total_added")
        .is_some()
    {
        out["expected"]["mana_added"] = json!(if def.name() == "Arixmethes, Slumbering Isle" {2}else{1});
        out["actual"]["mana_added"] = out["state_evidence"]["mana_action_total_added"].clone();
        out["expected"]["source_tapped"] = json!(true);
        out["actual"]["source_tapped"] = out["state_evidence"]["mana_action_source_tapped"].clone();
    }
    if matches!(def.name(), "Land Cap" | "Lava Tubes" | "River Delta" | "Timberline Ridge" | "Veldt") {
        out["expected"]["depletion_counters_after"] = json!(1);
        out["actual"]["depletion_counters_after"] = json!(g.counter_count(source,ironsmith::object::CounterType::Depletion));
    }
    out["expected"]["source_is_creature_before_activation"] = json!(condition_on);
    out["actual"]["source_is_creature_before_activation"] = out["state_evidence"]["source_is_creature_before_activation"].clone();
    Ok(out)
}
#[test]
#[ignore = "manual scoped activation-index report"]
fn report_ability_index_devotion() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inventory =
        root.join("reports/runtime-audit/corpus/267a16aff3b321196397d0b4/inventory.json");
    let candidates = root.join("reports/runtime-audit/ability-index-structural-candidates.json");
    let source =
        root.join("crates/ironsmith-tools/tests/runtime_ability_index_devotion_reproductions.rs");
    let binary = std::env::current_exe().unwrap();
    let paths = [&inventory, &candidates, &source, &binary];
    let before: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let inv: Value = serde_json::from_slice(&std::fs::read(&inventory).unwrap()).unwrap();
    let names: Vec<String> = ["Erebos, Bleak-Hearted", "Erebos, God of the Dead", "Heliod, God of the Sun", "Heliod, Sun-Crowned", "Nylea, Keen-Eyed", "Pharika, God of Affliction", "Purphoros, Bronze-Blooded", "Purphoros, God of the Forge", "Thassa, Deep-Dwelling", "Thassa, God of the Sea"]
    .iter()
    .map(|n| n.to_string())
    .collect();
    let mut compile = vec![];
    let mut defs = std::collections::HashMap::new();
    for name in names.iter().map(String::as_str).chain(["Twiddle"]) {
        let p = inv["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["name"] == name)
            .unwrap();
        let (artifact, def) = ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
            p["parse_input"].as_str().unwrap(),
            false,
        )
        .unwrap();
        compile.push(json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}));
        defs.insert(name.to_string(), def);
    }
    let mut rows = vec![];
    for name in names {
        if std::env::var("AUDIT_INDEX_CARD").is_ok_and(|selected| selected != name) {continue;}
        let def = &defs[&name];
        let activations = 1;
        for condition_on in [false, true] {
            if std::env::var("AUDIT_INDEX_CONDITION").is_ok_and(|selected| selected != condition_on.to_string()) {continue;}
            if condition_on && matches!(name.as_str(), "Land Cap" | "Lava Tubes" | "River Delta" | "Timberline Ridge" | "Veldt") {continue;}
            eprintln!("AUDIT_CASE {name} {condition_on}");
            for ordinal in 0..activations {
                let (status, out) = match run(def, &defs["Twiddle"], condition_on, ordinal)
                {
                    Ok(v) => {
                        let status = if v["actual"]["announcement_error"].is_string() {
                            "action_or_choice_failed"
                        } else if v["actual"]["resolution_error"].is_string() {
                            "resolution_failed"
                        } else if v["actual"] == v["expected"] {
                            "expected_result_observed"
                        } else {
                            "semantic_mismatch"
                        };
                        (status, v)
                    }
                    Err(e) => (
                        "fixture_or_execution_error",
                        json!({"expected":null,"actual":{"error":e}}),
                    ),
                };
                rows.push(json!({"card":name,"scenario":{"condition_on":condition_on,"source_activation_ordinal":ordinal},"status":status,"expected":out["expected"],"actual":out["actual"],"state_evidence":out["state_evidence"],"execution_trace":out["execution_trace"],"artifact_checksum":compile.iter().find(|c|c["card"]==name).unwrap()["artifact_checksum"],"scope":"Actual paid canonical God cast and advertised activation dispatch below and above printed devotion threshold. Established enchantment mana symbols provide actual measured devotion; source creature status is asserted before activation. Both variants provide legal sacrifice/target/graveyard resources and resolve through normal priority. Other activated effect results are outside this dispatch-focused report."}));
            }
        }
    }
    let after: Vec<_> = paths
        .iter()
        .map(|p| json!({"path":p,"sha256":hash(p)}))
        .collect();
    let report = json!({"rows":rows,"compilation":compile,"provenance":{"before":before,"after":after,"artifacts_unchanged":before==after,"strict_artifact":true,"unique_card_ids":true,"seed":71757432704855_u64}});
    std::fs::write(
        std::env::var("AUDIT_INDEX_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(|_|root.join("reports/runtime-audit/ability-index-devotion-reproductions.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("wrote{}cases", report["rows"].as_array().unwrap().len());
}

//! Opt-in canonical runtime evidence. Passing means reports were generated.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, LegalAction,
    SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, ViewCardsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_attacker_declarations_with_dm, apply_blocker_declarations,
    apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, PowerToughness, Step,
    Subtype, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
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
    accept: bool,
    trace: Vec<Value>,
    planar_viewed: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{c:?}"),"answer":self.accept}));
        self.accept
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(g, c);
        self.trace.push(json!({"choice":"objects","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(g, c);
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
fn finish(
    game: &mut GameState,
    q: &mut TriggerQueue,
    dm: &mut impl DecisionMaker,
) -> Result<(), String> {
    for _ in 0..32 {
        advance_priority_with_dm(game, q, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("resolution exceeded32 steps".into())
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
fn block(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    blocker: ObjectId,
    attacker: ObjectId,
    defender: PlayerId,
) -> Result<(), String> {
    game.turn.step = Some(Step::DeclareBlockers);
    let mut combat = game.combat.take().ok_or("missing combat")?;
    let result = apply_blocker_declarations(
        game,
        &mut combat,
        queue,
        &[BlockerDeclaration {
            blocker,
            blocking: attacker,
        }],
        defender,
    )
    .map_err(|e| e.to_string());
    game.combat = Some(combat);
    result
}
fn compile(names: &[String]) -> HashMap<String, (CardDefinition, String)> {
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        names,
    )
    .unwrap();
    names
        .iter()
        .map(|name| {
            let p = &payloads[name][0];
            let builder = ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                p.parse_name.as_deref().unwrap_or(&p.name),
            );
            let (a, d) =
                ironsmith_registry::compile_builder_to_artifact(builder, &p.parse_input, false)
                    .unwrap();
            (name.clone(), (d, a.payload_checksum))
        })
        .collect()
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
    let mut reader = std::fs::File::open(&path).unwrap();
    let mut h = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = reader.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    let report = json!({"scope":scope,"limitations":limitations,"provenance":{"binary":path,"binary_sha256":h.finalize().iter().map(|b|format!("{b:02x}")).collect::<String>(),"compiled_via":"ironsmith_registry::compile_builder_to_artifact","seed":SEED,"unique_card_ids":true},"rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit")
        .join(name);
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}
fn enable_planar(game: &mut GameState) -> Result<(), String> {
    let planes = (0..30)
        .map(|n| {
            (
                CardDefinitionBuilder::new(CardId::new(), &format!("Neutral plane {n}"))
                    .card_types(vec![CardType::Plane])
                    .build(),
                ironsmith::game_state::PlanarCardKind::Plane,
            )
        })
        .collect();
    game.enable_planechase_communal(planes)?;
    game.reveal_starting_plane()?;
    Ok(())
}

fn planar_case(
    def: &CardDefinition,
    start: &CardDefinition,
    planechase: bool,
    accept: bool,
    time_lord: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = game();
    if planechase {
        enable_planar(&mut game)?;
    }
    let face_before = game.face_up_planar_objects().to_vec();
    let next_plane = game
        .planar_deck(alice())
        .and_then(|deck| {
            deck.iter()
                .rev()
                .nth(usize::from(def.name() == "Susan Foreman"))
        })
        .copied();
    let mut dm = Choices {
        accept,
        trace: Vec::new(),
        planar_viewed: 0,
    };
    let mut primary = json!({});
    let result: Result<(), String> = (|| {
        if def.name() == "TARDIS" {
            // Established Vehicle controlled since an earlier turn. Its crew cost
            // and attack are performed now; a freshly cast Vehicle could not attack.
            let source = game.create_object_from_definition(def, alice(), Zone::Battlefield);
            game.remove_summoning_sickness(source);
            dm.trace.push(json!({"stage":"established_previous_turn_vehicle","source":format!("{source:?}"),"casting_cost_not_exercised":true}));
            let crew = CardDefinitionBuilder::new(CardId::new(), "TARDIS crew fixture")
                .card_types(vec![CardType::Creature])
                .subtypes(if time_lord {
                    vec![Subtype::TimeLord]
                } else {
                    vec![Subtype::Human]
                })
                .power_toughness(PowerToughness::fixed(2, 6))
                .build();
            let crew_id = game.create_object_from_definition(&crew, alice(), Zone::Battlefield);
            let action=compute_legal_actions(&game,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==2)).ok_or("no legal Crew2 activation")?;
            let mut queue = announce(&mut game, action, &mut dm)?;
            finish(&mut game, &mut queue, &mut dm)?;
            if !game
                .current_card_types(source)
                .is_some_and(|types| types.contains(&CardType::Creature))
            {
                return Err("crew did not animate TARDIS".into());
            }
            dm.trace.push(
                json!({"stage":"actual_crew_completed","crew_tapped":game.is_tapped(crew_id)}),
            );
            let mut queue = attack(&mut game, &[source], bob())?;
            let resolved = finish(&mut game, &mut queue, &mut dm);
            let cascade_grants=game.effect_store.temporary_spell_ability_grants.iter().filter(|grant|matches!(&grant.ability.kind,AbilityKind::Static(a)if a.id()==ironsmith::static_abilities::StaticAbilityId::Cascade)).count();
            primary = json!({"crewed":true,"crew_tapped":game.is_tapped(crew_id),"cascade_grants":cascade_grants});
            resolved?;
        } else {
            if def.name() == "Susan Foreman" {
                let mut queue = cast(&mut game, def, &mut dm)?;
                finish(&mut game, &mut queue, &mut dm)?;
            }
            for n in 0..4 {
                game.create_object_from_definition(
                    &creature(&format!("Surveil draw fixture {n}"), false, false),
                    alice(),
                    Zone::Library,
                );
            }
            let mut queue = cast(
                &mut game,
                if def.name() == "Susan Foreman" {
                    start
                } else {
                    def
                },
                &mut dm,
            )?;
            let resolved = finish(&mut game, &mut queue, &mut dm);
            primary = json!({"cards_drawn":game.player(alice()).unwrap().hand.len(),"library_remaining":game.player(alice()).unwrap().library.len()});
            resolved?;
        }
        Ok(())
    })();
    let did_walk = planechase && accept && (def.name() != "TARDIS" || time_lord);
    let actual = json!({"primary_effect":primary,"planeswalk_count":game.planeswalk_count(),"face_changed":game.face_up_planar_objects()!=face_before.as_slice(),"expected_plane_revealed":!did_walk||game.face_up_planar_objects().first().copied()==next_plane,"planar_cards_viewed":dm.planar_viewed});
    trace.extend(dm.trace);
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}
#[test]
#[ignore = "manual report generation; passing is not semantic correctness"]
fn report_remaining_planechase_contexts() {
    let names = ["Start the TARDIS", "Susan Foreman", "TARDIS"]
        .map(str::to_owned)
        .to_vec();
    let defs = compile(&names);
    let mut rows = Vec::new();
    for name in names {
        let (def, checksum) = &defs[&name];
        for planechase in [false, true] {
            for accept in [false, true] {
                let mut trace = Vec::new();
                let primary = if name == "TARDIS" {
                    json!({"crewed":true,"crew_tapped":true,"cascade_grants":1})
                } else {
                    json!({"cards_drawn":1,"library_remaining":3})
                };
                record(
                    &mut rows,
                    &name,
                    json!({"planechase":planechase,"accept_planeswalk":accept,"time_lord_present":name=="TARDIS"||name=="Susan Foreman","planeswalk_producer":if name=="Susan Foreman"{"canonical Start the TARDIS with Susan replacement"}else{"canonical card's own instruction"}}),
                    json!({"primary_effect":primary,"planeswalk_count":if planechase{Some(u64::from(accept))}else{None},"face_changed":planechase&&accept,"expected_plane_revealed":true,"planar_cards_viewed":if planechase&&accept&&name=="Susan Foreman"{2}else{0}}),
                    planar_case(
                        def,
                        &defs["Start the TARDIS"].0,
                        planechase,
                        accept,
                        true,
                        &mut trace,
                    ),
                    checksum,
                    trace,
                );
            }
        }
        if name == "TARDIS" {
            for planechase in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"planechase":planechase,"accept_planeswalk":true,"time_lord_present":false}),
                    json!({"primary_effect":{"crewed":true,"crew_tapped":true,"cascade_grants":0},"planeswalk_count":if planechase{Some(0)}else{None},"face_changed":false,"expected_plane_revealed":true,"planar_cards_viewed":0}),
                    planar_case(
                        def,
                        &defs["Start the TARDIS"].0,
                        planechase,
                        true,
                        false,
                        &mut trace,
                    ),
                    checksum,
                    trace,
                );
            }
        }
    }
    report(
        "remaining-planechase-reproductions.json",
        "Fourteen actual spell-cast and established-Vehicle crew/attack scenarios across remaining three planeswalk cards",
        "Ordinary-game planeswalk instructions should do nothing (official Doctor Who release notes). Planechase uses thirty neutral planes, active caster/planar controller. Susan is exercised through canonical Start the TARDIS and needs interaction attribution. TARDIS is an established previous-turn Vehicle: its casting cost is not exercised, but crew, actual attack trigger and cascade grant registration are checked; later cascade resolution and out-of-turn casting are not tested. Start the TARDIS jump-start is not tested.",
        rows,
    );
}

fn combat_case(def: &CardDefinition, kind: &str, trace: &mut Vec<Value>) -> Result<Value, String> {
    let mut game = game();
    let source_zone = if def.name() == "Nemesis Phoenix" {
        Zone::Graveyard
    } else {
        Zone::Battlefield
    };
    let source = game.create_object_from_definition(def, alice(), source_zone);
    game.remove_summoning_sickness(source);
    let index=def.abilities.iter().position(|a|matches!(&a.kind,AbilityKind::Activated(a)if a.additional_restrictions.iter().any(|r|r.contains("only if")))).ok_or("no condition-bearing activated ability")?;
    let blue = kind.contains("blue") && !kind.contains("nonblue");
    let opponent_flying = kind == "opponent_flying";
    let other = game.create_object_from_definition(
        &creature("Other friendly attacker", false, kind == "own_flying"),
        alice(),
        Zone::Battlefield,
    );
    let enemy_definition = if def.name() == "Grizzled Wolverine" {
        // Keep the Wolverine alive in the later-step boundary fixture even after
        // the combat damage turn-based action; damage resolution itself is omitted.
        CardDefinitionBuilder::new(CardId::new(), "Zero-power blocker fixture")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 6))
            .build()
    } else {
        creature("Opponent combat fixture", blue, opponent_flying)
    };
    let enemy = game.create_object_from_definition(&enemy_definition, bob(), Zone::Battlefield);
    for n in 0..3 {
        game.create_object_from_definition(
            &creature(&format!("Discard buffer {n}"), false, false),
            alice(),
            Zone::Hand,
        );
        game.create_object_from_definition(
            &creature(&format!("Draw buffer {n}"), false, false),
            alice(),
            Zone::Library,
        );
    }
    let mut dm = Choices {
        accept: true,
        trace: Vec::new(),
        planar_viewed: 0,
    };
    let mut entering_combat = !matches!(
        kind,
        "outside_combat" | "outside_with_snow" | "no_flying" | "own_flying" | "opponent_flying"
    );
    if def.name() == "Groundling Pouncer" {
        entering_combat = false;
    }
    if kind.contains("snow") && !kind.contains("no_snow") {
        let land = CardDefinitionBuilder::new(CardId::new(), "Defending snow land")
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Snow])
            .build();
        game.create_object_from_definition(&land, bob(), Zone::Battlefield);
    }
    if def.name() == "Kongming's Contraptions" {
        game.turn.active_player = bob();
        let defender = if kind == "attacks_someone_else" {
            PlayerId(2)
        } else {
            alice()
        };
        let mut queue = attack(&mut game, &[enemy], defender)?;
        finish(&mut game, &mut queue, &mut dm)?;
        if kind == "attacked_you_after_step" {
            game.turn.step = Some(Step::DeclareBlockers);
        }
        entering_combat = false;
    }
    if def.name() == "Nemesis Phoenix" {
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.remove_summoning_sickness(other);
        let mut declarations = vec![AttackerDeclaration {
            creature: other,
            target: AttackTarget::Player(bob()),
        }];
        if kind != "one_opponent" {
            let second = game.create_object_from_definition(
                &creature("Second friendly attacker", false, false),
                alice(),
                Zone::Battlefield,
            );
            game.remove_summoning_sickness(second);
            let defender = if kind == "two_creatures_one_opponent" {
                bob()
            } else {
                PlayerId(2)
            };
            declarations.push(AttackerDeclaration {
                creature: second,
                target: AttackTarget::Player(defender),
            });
        }
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        apply_attacker_declarations_with_dm(
            &mut game,
            &mut combat,
            &mut queue,
            &declarations,
            &mut dm,
        )
        .map_err(|e| e.to_string())?;
        game.combat = Some(combat);
        finish(&mut game, &mut queue, &mut dm)?;
        if kind == "two_opponents_after_step" {
            game.turn.step = Some(Step::DeclareBlockers);
        }
        entering_combat = false;
    }
    if entering_combat {
        let mut queue;
        let source_blocks = matches!(kind, "source_blocking" | "blue_source_blocks");
        if source_blocks {
            game.turn.active_player = bob();
            queue = attack(&mut game, &[enemy], alice())?;
            block(&mut game, &mut queue, source, enemy, alice())?;
        } else {
            let source_attacks = matches!(
                kind,
                "source_attacking"
                    | "source_unblocked"
                    | "source_blocked"
                    | "source_blocked_after_step"
                    | "blue_source_blocked"
                    | "nonblue_source_blocked"
                    | "blue_blocked_after_combat"
            );
            queue = attack(
                &mut game,
                &[if source_attacks { source } else { other }],
                bob(),
            )?;
            if matches!(
                kind,
                "source_blocked"
                    | "source_blocked_after_step"
                    | "blue_source_blocked"
                    | "nonblue_source_blocked"
                    | "blue_blocked_after_combat"
            ) {
                block(&mut game, &mut queue, enemy, source, bob())?;
            }
            if kind == "source_unblocked" {
                game.turn.step = Some(Step::DeclareBlockers);
            }
            if kind == "source_blocked_after_step" {
                game.turn.step = Some(Step::CombatDamage);
            }
        }
        finish(&mut game, &mut queue, &mut dm)?;
        if kind == "blue_blocked_after_combat" {
            game.turn.phase = Phase::NextMain;
            game.turn.step = None;
            game.combat = None;
        }
    }
    game.turn.priority_player = Some(alice());
    trace.push(json!({"stage":"established_state","source":format!("{source:?}"),"ability_index":index,"ability":format!("{:?}",game.current_activated_ability(source,index)),"phase":format!("{:?}",game.turn.phase),"step":format!("{:?}",game.turn.step),"combat":format!("{:?}",game.combat),"history":format!("{:?}",game.turn_store.turn_history),"source_tapped":game.is_tapped(source)}));
    let action=compute_legal_actions(&game,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,ability_index}if *id==source&&*ability_index==index));
    let offered = action.is_some();
    let mut announced = false;
    if let Some(action) = action {
        trace.push(json!({"stage":"offered_action","action":format!("{action:?}")}));
        let before = game.player(alice()).unwrap().mana_pool.total();
        let result = announce(&mut game, action, &mut dm);
        trace.extend(dm.trace);
        result?;
        announced = game.stack.iter().any(|entry| entry.object_id == source);
        trace.push(json!({"stage":"announcement_completed","mana_paid":before.saturating_sub(game.player(alice()).unwrap().mana_pool.total()),"stack":format!("{:?}",game.stack)}));
    } else {
        trace.extend(dm.trace);
    }
    Ok(json!({"activation_offered":offered,"activation_announced":announced}))
}
#[test]
#[ignore = "manual report generation; passing is not semantic correctness"]
fn report_combat_activation_gates() {
    let mut cases = Vec::new();
    for name in [
        "Ancient Hellkite",
        "Forgestoker Dragon",
        "Gerrard Capashen",
        "Glint-Horn Buccaneer",
    ] {
        cases.extend([
            (name, "source_idle", false),
            (name, "source_attacking", true),
        ]);
    }
    cases.extend([
        ("Cinder Crawler", "source_unblocked", false),
        ("Cinder Crawler", "source_blocked", true),
        ("Sawback Manticore", "source_idle", false),
        ("Sawback Manticore", "source_attacking", true),
        ("Sawback Manticore", "source_blocking", true),
        ("Grizzled Wolverine", "source_unblocked", false),
        ("Grizzled Wolverine", "source_blocked", true),
        ("Grizzled Wolverine", "source_blocked_after_step", false),
        ("Groundling Pouncer", "no_flying", false),
        ("Groundling Pouncer", "own_flying", false),
        ("Groundling Pouncer", "opponent_flying", true),
        ("Arcum's Sleigh", "outside_with_snow", false),
        ("Arcum's Sleigh", "combat_no_snow", false),
        ("Arcum's Sleigh", "combat_snow", true),
        ("Kjeldoran Guard", "outside_combat", false),
        ("Kjeldoran Guard", "combat_snow", false),
        ("Kjeldoran Guard", "combat_no_snow", true),
        ("Sea Troll", "source_idle", false),
        ("Sea Troll", "nonblue_source_blocked", false),
        ("Sea Troll", "blue_source_blocked", true),
        ("Sea Troll", "blue_source_blocks", true),
        ("Sea Troll", "blue_blocked_after_combat", true),
    ]);
    let mut names = cases
        .iter()
        .map(|(n, _, _)| n.to_string())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    let defs = compile(&names);
    let mut rows = Vec::new();
    for (name, kind, allowed) in cases {
        let (def, checksum) = &defs[name];
        let mut trace = Vec::new();
        let result = combat_case(def, kind, &mut trace);
        record(
            &mut rows,
            name,
            json!({"fixture":kind,"authored_condition_should_allow":allowed}),
            json!({"activation_offered":allowed,"activation_announced":allowed}),
            result,
            checksum,
            trace,
        );
    }
    report(
        "combat-activation-gate-reproductions.json",
        "Thirty actual legal-action discovery and paid activation-announcement cases across eleven combat-gated canonical cards",
        "Established source permanents use direct battlefield fixture placement and remove summoning sickness; attack/block states and blue-block history come from real legal declarations. Conditions are checked before first activation only, so once-per-turn limits are not covered. Sufficient mana, legal targets, discard resources and libraries prevent unrelated cost failures. Announced abilities remain on the stack; this report proves legality/announcement behavior, not resolution outcomes. Later-step fixtures advance phase without dealing combat damage; Wolverine uses a zero-power blocker to preserve its legal survival, and the post-combat Troll history case clears combat. Ancient Hellkite absence may also depend on defining its defending-player target, so its successful negative case does not independently prove the authored attacking gate.",
        rows,
    );
}

#[test]
#[ignore = "manual report generation; passing is not semantic correctness"]
fn report_remaining_attack_history_gates() {
    let cases = [
        ("Kongming's Contraptions", "attacks_someone_else", false),
        ("Kongming's Contraptions", "attacked_you", true),
        ("Kongming's Contraptions", "attacked_you_after_step", false),
        ("Nemesis Phoenix", "one_opponent", false),
        ("Nemesis Phoenix", "two_creatures_one_opponent", false),
        ("Nemesis Phoenix", "two_opponents", true),
        ("Nemesis Phoenix", "two_opponents_after_step", false),
    ];
    let names = vec!["Kongming's Contraptions".into(), "Nemesis Phoenix".into()];
    let defs = compile(&names);
    let mut rows = Vec::new();
    for (name, kind, allowed) in cases {
        let (def, checksum) = &defs[name];
        let mut trace = Vec::new();
        let result = combat_case(def, kind, &mut trace);
        record(
            &mut rows,
            name,
            json!({"fixture":kind,"authored_condition_should_allow":allowed}),
            json!({"activation_offered":allowed,"activation_announced":allowed}),
            result,
            checksum,
            trace,
        );
    }
    report(
        "remaining-attack-history-gate-reproductions.json",
        "Seven actual attack-declaration and paid activation-announcement scenarios for two conditional gates",
        "Kongming is an established untapped battlefield source; Nemesis Phoenix starts in its printed graveyard activation zone. Actual attack declarations distinguish defending player, one versus two opponents, and two attackers attacking the same opponent. Later declare-blockers boundary adjusts step with no declared blockers; no combat damage is performed. Announced effects remain on stack; return-to-battlefield and damage resolution are outside scope.",
        rows,
    );
}

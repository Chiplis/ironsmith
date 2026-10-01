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
use ironsmith::mana::{ManaCost, ManaSymbol};
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
    for step in 0..32 {
        eprintln!(
            "MODAL_STAGE finish step={step} stack_len={}",
            game.stack.len()
        );
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
    let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/modal-candidate-inputs.json");
    let document: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    names
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
    let binary_hash = std::env::var("AUDIT_BINARY_SHA256")
        .expect("isolating driver supplies independently verified executable hash");
    let report = json!({"scope":scope,"limitations":limitations,"provenance":{"binary":path,"binary_sha256":binary_hash,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","seed":SEED,"unique_card_ids":true},"rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit")
        .join(name);
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct ModalChoices {
    inner: Choices,
    modes: Vec<usize>,
    x: u32,
    targets: Vec<ironsmith::game_state::Target>,
}
impl DecisionMaker for ModalChoices {
    fn decide_number(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let n = self.x.clamp(c.min, c.max);
        self.inner
            .trace
            .push(json!({"choice":"number","context":format!("{c:?}"),"selected":n}));
        n
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if c.description.to_ascii_lowercase().contains("mode") {
            self.inner
                .trace
                .push(json!({"choice":"modes","context":format!("{c:?}"),"selected":self.modes}));
            self.modes.clone()
        } else {
            self.inner.decide_options(g, c)
        }
    }
    fn decide_targets(
        &mut self,
        _: &GameState,
        c: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let selected = c
            .requirements
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                self.targets
                    .get(i)
                    .copied()
                    .filter(|t| r.legal_targets.contains(t))
            })
            .collect::<Vec<_>>();
        self.inner.trace.push(json!({"choice":"explicit_mode_targets","context":format!("{c:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_boolean(&mut self, g: &GameState, c: &BooleanContext) -> bool {
        self.inner.decide_boolean(g, c)
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        self.inner.decide_objects(g, c)
    }
}
fn choices(modes: Vec<usize>, x: u32) -> ModalChoices {
    ModalChoices {
        inner: Choices {
            target: None,
            accept: true,
            trace: Vec::new(),
            planar_viewed: 0,
        },
        modes,
        x,
        targets: Vec::new(),
    }
}
fn cast_for(
    game: &mut GameState,
    def: &CardDefinition,
    player: PlayerId,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    game.turn.priority_player = Some(player);
    let id = game.create_object_from_definition(def, player, Zone::Hand);
    let action = compute_legal_actions(game, player).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==id))
        .ok_or("no legal requested cast")?;
    announce(game, action, dm)
}
fn doomsday(
    def: &CardDefinition,
    modes: Vec<usize>,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = game();
    for player in [alice(), bob(), PlayerId(2)] {
        for n in 0..3 {
            game.create_object_from_definition(
                &creature(&format!("Doom sacrifice creature {n}"), false, false),
                player,
                Zone::Battlefield,
            );
        }
        let guard = CardDefinitionBuilder::new(CardId::new(), "Doom artifact creature guard")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 6))
            .build();
        game.create_object_from_definition(&guard, player, Zone::Battlefield);
        for n in 0..3 {
            game.create_object_from_definition(
                &creature(&format!("Doom discard buffer {n}"), false, false),
                player,
                Zone::Hand,
            );
        }
    }
    let mut dm = choices(modes.clone(), modes.len() as u32);
    let before = game.player(alice()).unwrap().mana_pool.total();
    let mut paid = 0;
    let result: Result<(), String> = (|| {
        let mut queue = cast_for(&mut game, def, alice(), &mut dm)?;
        paid = before.saturating_sub(game.player(alice()).unwrap().mana_pool.total());
        dm.inner
            .trace
            .push(json!({"stage":"announced","mana_paid":paid,"stack":format!("{:?}",game.stack)}));
        finish(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    let daleks = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|o| o.subtypes.contains(&Subtype::Dalek))
        })
        .collect::<Vec<_>>();
    let actual = json!({"mana_paid":paid,"sacrifices_per_player":([0,1,2].map(|i|game.player(PlayerId(i)).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Doom sacrifice"))).count())),"artifact_guards_survive":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|o|o.name=="Doom artifact creature guard")).count(),"dalek_count":daleks.len(),"daleks_match_characteristics":daleks.iter().all(|id|game.calculated_power(*id)==Some(3)&&game.calculated_toughness(*id)==Some(3)&&game.current_colors(*id)==Some(ironsmith::color::ColorSet::BLACK)&&game.current_card_types(*id).is_some_and(|t|t.contains(&CardType::Artifact)&&t.contains(&CardType::Creature))&&game.object_has_static_ability_id(*id,ironsmith::static_abilities::StaticAbilityId::Menace)),"hand_counts":([0,1,2].map(|i|game.player(PlayerId(i)).unwrap().hand.len()))});
    trace.extend(dm.inner.trace);
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}
fn silumgar(
    def: &CardDefinition,
    modes: Vec<usize>,
    shared: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = game();
    game.turn.active_player = bob();
    game.turn.priority_player = Some(bob());
    let artifact = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Command artifact target")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob(),
        Zone::Battlefield,
    );
    let creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Command creature target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        bob(),
        Zone::Battlefield,
    );
    let walker = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Command planeswalker target")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build(),
        bob(),
        Zone::Battlefield,
    );
    let identities = [artifact, creature, walker].map(|id| game.object(id).unwrap().stable_id);
    for n in 0..3 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), &format!("Incoming draw buffer {n}"))
                .card_types(vec![CardType::Land])
                .build(),
            bob(),
            Zone::Library,
        );
    }
    let incoming = CardDefinitionBuilder::new(CardId::new(), "Incoming noncreature spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .with_spell_effect(vec![ironsmith::Effect::draw(1)])
        .build();
    let mut incoming_dm = choices(Vec::new(), 0);
    let mut incoming_queue = cast_for(&mut game, &incoming, bob(), &mut incoming_dm)?;
    trace.extend(incoming_dm.inner.trace);
    let spell = game
        .stack
        .last()
        .ok_or("incoming spell not on stack")?
        .object_id;
    let bounce = if shared {
        if modes.contains(&2) { creature } else { walker }
    } else {
        artifact
    };
    let mut dm = choices(modes.clone(), 0);
    dm.targets = modes
        .iter()
        .map(|mode| {
            ironsmith::game_state::Target::Object(match mode {
                0 => spell,
                1 => bounce,
                2 => creature,
                3 => walker,
                _ => unreachable!(),
            })
        })
        .collect();
    let before = game.player(alice()).unwrap().mana_pool.total();
    let mut paid = 0;
    let result: Result<(), String> = (|| {
        let mut queue = cast_for(&mut game, def, alice(), &mut dm)?;
        paid = before.saturating_sub(game.player(alice()).unwrap().mana_pool.total());
        dm.inner.trace.push(json!({"stage":"command_announced","mana_paid":paid,"stack":format!("{:?}",game.stack)}));
        finish(&mut game, &mut queue, &mut dm)?;
        finish(&mut game, &mut incoming_queue, &mut dm)?;
        Ok(())
    })();
    let ids = identities.map(|s| game.find_object_by_stable_id(s));
    let zones = ids.map(|id| {
        id.and_then(|id| game.object(id))
            .map(|o| format!("{:?}", o.zone))
    });
    let actual = json!({"mana_paid":paid,"target_zones_artifact_creature_planeswalker":zones,"creature_pt_if_on_battlefield":ids[1].filter(|id|game.object(*id).is_some_and(|o|o.zone==Zone::Battlefield)).map(|id|[game.calculated_power(id).unwrap(),game.calculated_toughness(id).unwrap()]),"incoming_spell_drew":game.player(bob()).unwrap().hand.iter().any(|id|game.object(*id).is_some_and(|o|o.name.starts_with("Incoming draw buffer"))),"stack_empty":game.stack.is_empty()});
    trace.extend(dm.inner.trace);
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}
#[test]
#[ignore = "manual report generation; passing is not semantic correctness"]
fn report_modal_candidates() {
    let names = vec!["Doomsday Confluence".into(), "Silumgar's Command".into()];
    let defs = compile(&names);
    let mut rows = Vec::new();
    for modes in [
        vec![],
        vec![0],
        vec![1],
        vec![2],
        vec![0, 0],
        vec![1, 1],
        vec![2, 2],
        vec![0, 1, 2],
        vec![0, 0, 0],
        vec![1, 1, 1],
        vec![2, 2, 2],
        vec![1, 1, 1, 1],
    ] {
        let (def, checksum) = &defs["Doomsday Confluence"];
        let sacrifices = modes.iter().filter(|m| **m == 0).count();
        let tokens = modes.iter().filter(|m| **m == 1).count();
        let discard = modes.iter().filter(|m| **m == 2).count();
        let mut trace = Vec::new();
        let result = doomsday(def, modes.clone(), &mut trace);
        record(
            &mut rows,
            "Doomsday Confluence",
            json!({"modes":modes,"requested_x":modes.len(),"three_nonartifact_creatures_each":true,"three_hand_cards_each":true}),
            json!({"mana_paid":2*modes.len()+1,"sacrifices_per_player":[sacrifices,sacrifices,sacrifices],"artifact_guards_survive":3,"dalek_count":tokens,"daleks_match_characteristics":true,"hand_counts":[3,3-discard,3-discard]}),
            result,
            checksum,
            trace,
        );
        let row = rows.last_mut().unwrap();
        if row["status"] == "execution_or_fixture_error"
            && row["actual"]["error"].as_str().is_some_and(|e| {
                e.contains("mode selection has no legal joint optional-cost proposal")
            })
        {
            let raw_error = row["actual"].clone();
            row["requested_resolution_expected"] = row["expected"].clone();
            row["expected"] = json!({"card_rule_allows_requested_mode_count":true,"mode_selection_accepted":true});
            row["actual"] = json!({"card_rule_allows_requested_mode_count":true,"mode_selection_accepted":false,"engine_rejection":raw_error});
            row["status"] = json!("semantic_mismatch");
            row["outcome_category"] = json!("silent_wrong_result");
            row["outcome_scope"] = json!("missing_legal_cast_choice");
            row["review_note"] = json!(
                "The mode prompt requires exactly three points before asking X. This valid card-rule mode count cannot be submitted through that prompt. The attempted response is outside the incorrect UI limits, so this row confirms missing legal choice availability, not a paid resolution failure; X selection and payment were not reached."
            );
        }
    }
    for (modes, shared) in [
        (vec![0, 1], false),
        (vec![0, 2], false),
        (vec![0, 3], false),
        (vec![1, 2], false),
        (vec![1, 3], false),
        (vec![2, 3], false),
        (vec![1, 2], true),
        (vec![1, 3], true),
    ] {
        let (def, checksum) = &defs["Silumgar's Command"];
        let mut zones = ["Battlefield"; 3];
        if modes.contains(&1) {
            zones[if shared {
                if modes.contains(&2) { 1 } else { 2 }
            } else {
                0
            }] = "Hand";
        }
        if modes.contains(&3) && zones[2] == "Battlefield" {
            zones[2] = "Graveyard";
        }
        let pt = if zones[1] == "Battlefield" {
            Some(if modes.contains(&2) { [1, 1] } else { [4, 4] })
        } else {
            None
        };
        let mut trace = Vec::new();
        let result = silumgar(def, modes.clone(), shared, &mut trace);
        record(
            &mut rows,
            "Silumgar's Command",
            json!({"modes":modes,"bounce_and_later_mode_share_target":shared,"actual_opponent_noncreature_spell_on_stack":true}),
            json!({"mana_paid":5,"target_zones_artifact_creature_planeswalker":zones,"creature_pt_if_on_battlefield":pt,"incoming_spell_drew":!modes.contains(&0),"stack_empty":true}),
            result,
            checksum,
            trace,
        );
    }
    report(
        "modal-candidate-reproductions.json",
        "Twenty paid-cast or explicit missing-legal-choice scenarios with modal choices, X, targets and resources",
        "Doomsday requests X0, each mode once/twice/thrice, all three with X3, and four token modes with X4. Cases rejected before X/payment are expressly legality observations, not paid-resolution errors. Silumgar tests all six mode pairs with distinct appropriate targets, plus legal shared-target bounce/modify and bounce/destroy pairs; later effects must tolerate targets moved by an earlier mode. All incoming spells are legally cast, and Command responds while they remain on stack. Successful report generation is not a semantics pass.",
        rows,
    );
}

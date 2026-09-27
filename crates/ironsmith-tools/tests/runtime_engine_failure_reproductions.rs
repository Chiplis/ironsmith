//! Opt-in expected-result audit of cards implicated by engine unit failures.
//! A successful test run means the report was produced; individual report rows
//! determine whether the observed card behavior met the expected result.

use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decision::{
    AttackerDeclaration, DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker,
    compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_attacker_declarations_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events,
    generate_and_queue_step_triggers, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::object::CounterType;
use ironsmith::rules::state_based::apply_state_based_actions_with;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, Phase, PlayerId, Step, Subtype, Zone,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

const SEED: u64 = 0x4952_4f4e_534d_4954;
fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(SEED);
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn creature(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

// Drive only observed priority decision types. An unknown decision is a fixture
// limitation, never an execution pass. All actions originate in legal_actions.
fn announce(game: &mut GameState, action: LegalAction) -> Result<TriggerQueue, String> {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut response = PriorityResponse::PriorityAction(action);
    for _ in 0..24 {
        let progress =
            apply_priority_response_with_dm(game, &mut queue, &mut state, &response, &mut dm)
                .map_err(|e| e.to_string())?;
        if !game.stack.is_empty()
            && state.pending_activation.is_none()
            && state.pending_cast.is_none()
        {
            return Ok(queue);
        }
        response = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ctx)) => {
                PriorityResponse::NextCostChoice(
                    ctx.options
                        .iter()
                        .find(|o| o.legal)
                        .ok_or("no legal cost option")?
                        .index,
                )
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectObjects(ctx)) => {
                PriorityResponse::CardCostChoice(
                    ctx.candidates
                        .iter()
                        .find(|o| o.legal)
                        .ok_or("no legal cost object")?
                        .id,
                )
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx))
                if ctx.plan.payable =>
            {
                PriorityResponse::ManaPaymentPlan(ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                })
            }
            other => {
                return Err(format!(
                    "unsupported announcement fixture decision: {other:?}"
                ));
            }
        };
    }
    Err("announcement exceeded 24 decisions".into())
}

fn ninjutsu(definition: &CardDefinition) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Hand);
    let attacker = game.create_object_from_definition(
        &creature("Returning attacker", 2),
        alice(),
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(attacker);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let defender = AttackTarget::Player(PlayerId::from_index(1));
    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: attacker,
            target: defender.clone(),
        }],
        ..CombatState::default()
    });
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 2);
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
        .ok_or("canonical ninjutsu ability was not a legal action")?;
    let _queue = announce(&mut game, action)?;
    let stale_after_payment = game
        .combat
        .as_ref()
        .unwrap()
        .attackers
        .iter()
        .any(|a| a.creature == attacker);
    let returned_to_hand = game.player(alice()).unwrap().hand.iter().any(|id| {
        game.object(*id)
            .is_some_and(|o| o.name == "Returning attacker")
    });
    if game.stack.len() != 1 {
        return Err(format!("unexpected stack size: {}", game.stack.len()));
    }
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .map_err(|e| e.to_string())?;
    let entered = game.battlefield.iter().copied().find(|id| {
        game.object(*id)
            .is_some_and(|o| o.name == definition.name())
    });
    Ok(json!({
        "returned_to_hand": returned_to_hand,
        "returned_object_still_attacks_after_payment": stale_after_payment,
        "attackers_after_resolution": game.combat.as_ref().unwrap().attackers.len(),
        "ninja_entered_tapped_and_attacking_defender": entered.is_some_and(|id|
            game.is_tapped(id) && game.combat.as_ref().unwrap().attackers.iter()
                .any(|a| a.creature == id && a.target == defender)),
    }))
}

fn grant_entry_counters(
    definition: &CardDefinition,
    mana_value: u8,
    ingredients: u32,
    mana_colors: &[ManaSymbol],
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    if ingredients > 0 {
        game.add_counters(source, CounterType::Named("ingredient".into()), ingredients);
    }
    let name = "Cast creature fixture";
    let spell =
        game.create_object_from_definition(&creature(name, mana_value), alice(), Zone::Hand);
    for (index, color) in mana_colors.iter().copied().enumerate() {
        let amount = if index == 0 {
            mana_value as u32 + 1 - mana_colors.len() as u32
        } else {
            1
        };
        game.player_mut(alice())
            .unwrap()
            .mana_pool
            .add(color, amount);
    }
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .ok_or("creature fixture was not a legal cast")?;
    let mut queue = announce(&mut game, action)?;
    let mut dm = SelectFirstDecisionMaker;
    let mut resolved_entries = 0;
    for _ in 0..12 {
        drain_pending_trigger_events(&mut game, &mut queue);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            break;
        }
        resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
        resolved_entries += 1;
    }
    if !game.stack.is_empty() {
        return Err("resolution exceeded 12 entries".into());
    }
    let entered = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| o.name == name))
        .ok_or("cast creature did not enter")?;
    Ok(
        json!({"plus_one_counters": game.counter_count(entered, CounterType::PlusOnePlusOne),
              "resolved_stack_entries": resolved_entries}),
    )
}

#[derive(Default)]
struct ChooseLossDestination {
    option: usize,
    descriptions: Vec<String>,
}
impl DecisionMaker for ChooseLossDestination {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.descriptions
            .push(format!("{:?}: {:?}", ctx.player, ctx.options));
        vec![
            ctx.options
                .iter()
                .filter(|o| o.legal)
                .nth(self.option)
                .or_else(|| ctx.options.iter().find(|o| o.legal))
                .expect("no legal replacement option")
                .index,
        ]
    }
}

fn simultaneous_loss(
    definition: &CardDefinition,
    lethal_damage: bool,
    option: usize,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    game.player_mut(alice()).unwrap().life = 0;
    if lethal_damage {
        game.mark_damage(source, 5);
    }
    let mut dm = ChooseLossDestination {
        option,
        ..Default::default()
    };
    if !apply_state_based_actions_with(&mut game, &mut dm) {
        return Err("no state-based action applied".into());
    }
    let in_zone = |zone| {
        game.objects_in_zone(zone)
            .iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|o| o.name == definition.name())
            })
            .count()
    };
    Ok(
        json!({"player_in_game": game.player(alice()).unwrap().is_in_game(),
        "life": game.player(alice()).unwrap().life,
        "angel_in_exile": in_zone(Zone::Exile), "angel_in_graveyard": in_zone(Zone::Graveyard)}),
    )
}

fn resolve_queue(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    dm: &mut dyn DecisionMaker,
) -> Result<usize, String> {
    let mut resolved = 0;
    for _ in 0..24 {
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
        resolved += 1;
    }
    Err("fixture exceeded 24 stack resolutions".into())
}

fn attack(game: &mut GameState, attackers: &[ObjectId]) -> Result<TriggerQueue, String> {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    for id in attackers {
        game.remove_summoning_sickness(*id);
    }
    let declarations = attackers
        .iter()
        .map(|id| AttackerDeclaration {
            creature: *id,
            target: AttackTarget::Player(PlayerId::from_index(1)),
        })
        .collect::<Vec<_>>();
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

fn aclazotz(definition: &CardDefinition, opponent_hands: [usize; 2]) -> Result<Value, String> {
    let mut game = setup();
    let card = creature("Nonland card fixture", 2);
    for _ in 0..8 {
        game.create_object_from_definition(&card, alice(), Zone::Library);
    }
    for (seat, count) in opponent_hands.iter().enumerate() {
        for _ in 0..*count {
            game.create_object_from_definition(
                &card,
                PlayerId::from_index(seat as u8 + 1),
                Zone::Hand,
            );
        }
    }
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let mut queue = attack(&mut game, &[source])?;
    let resolved = resolve_queue(&mut game, &mut queue, &mut SelectFirstDecisionMaker)?;
    Ok(json!({"drawn":game.player(alice()).unwrap().hand.len(),
        "opponent_hands":game.players.iter().skip(1).map(|p|p.hand.len()).collect::<Vec<_>>(),
        "opponent_graveyards":game.players.iter().skip(1).map(|p|p.graveyard.len()).collect::<Vec<_>>(),
        "resolved_stack_entries":resolved}))
}

fn alpine(
    definition: &CardDefinition,
    mountain: &CardDefinition,
    destination: Zone,
) -> Result<Value, String> {
    if definition.card.id == mountain.card.id {
        return Err("invalid fixture: source and Mountain share a CardId".into());
    }
    let mut game = setup();
    game.create_object_from_definition(mountain, alice(), Zone::Battlefield);
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    game.move_object_by_effect(source, destination)
        .ok_or("source zone transition failed")?;
    let resolved = resolve_queue(
        &mut game,
        &mut TriggerQueue::new(),
        &mut SelectFirstDecisionMaker,
    )?;
    let count = |zone| {
        game.objects_in_zone(zone)
            .iter()
            .filter(|id| game.object(**id).is_some_and(|o| o.name == mountain.name()))
            .count()
    };
    Ok(json!({"mountains_in_play":count(Zone::Battlefield),
        "mountains_in_graveyard":count(Zone::Graveyard),"resolved_stack_entries":resolved}))
}

struct BraidsChoices {
    accept_controller: bool,
    accept_bob: bool,
}
impl DecisionMaker for BraidsChoices {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        if ctx.player == alice() {
            self.accept_controller
        } else {
            ctx.player == PlayerId::from_index(1) && self.accept_bob
        }
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut legal = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .collect::<Vec<_>>();
        legal.sort_by_key(|c| !c.name.contains("artifact fixture"));
        legal
            .into_iter()
            .take(ctx.max.unwrap_or(1))
            .map(|c| c.id)
            .collect()
    }
}

fn braids(
    definition: &CardDefinition,
    accept_controller: bool,
    accept_bob: bool,
) -> Result<Value, String> {
    let mut game = setup();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Sacrificable artifact fixture")
        .card_types(vec![CardType::Artifact])
        .build();
    for seat in [0, 1] {
        game.create_object_from_definition(
            &artifact,
            PlayerId::from_index(seat),
            Zone::Battlefield,
        );
    }
    for _ in 0..8 {
        game.create_object_from_definition(&artifact, alice(), Zone::Library);
    }
    game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut queue);
    let resolved = resolve_queue(
        &mut game,
        &mut queue,
        &mut BraidsChoices {
            accept_controller,
            accept_bob,
        },
    )?;
    Ok(json!({"drawn":game.player(alice()).unwrap().hand.len(),
        "opponent_life":game.players.iter().skip(1).map(|p|p.life).collect::<Vec<_>>(),
        "graveyards":game.players.iter().map(|p|p.graveyard.len()).collect::<Vec<_>>(),
        "resolved_stack_entries":resolved}))
}

struct ChocoChoices;
impl DecisionMaker for ChocoChoices {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let mut legal = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .collect::<Vec<_>>();
        legal.sort_by_key(|c| c.name != "Bird attacker fixture");
        legal
            .into_iter()
            .take(ctx.max.unwrap_or(ctx.candidates.len()))
            .map(|c| c.id)
            .collect()
    }
}

fn choco(definition: &CardDefinition, land: &CardDefinition) -> Result<Value, String> {
    if definition.card.id == land.card.id {
        return Err("invalid fixture: source and land share a CardId".into());
    }
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let bird = CardDefinitionBuilder::new(CardId::new(), "Bird attacker fixture")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bird])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other = game.create_object_from_definition(&bird, alice(), Zone::Battlefield);
    // Library top is the final element. Choose the nonland for hand, then the land for battlefield.
    game.create_object_from_definition(land, alice(), Zone::Library);
    game.create_object_from_definition(&bird, alice(), Zone::Library);
    let mut queue = attack(&mut game, &[source, other])?;
    let resolved = resolve_queue(&mut game, &mut queue, &mut ChocoChoices)?;
    let lands = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|o| o.name == land.name()))
        .collect::<Vec<_>>();
    Ok(
        json!({"hand":game.player(alice()).unwrap().hand.len(),"library":game.player(alice()).unwrap().library.len(),
        "graveyard":game.player(alice()).unwrap().graveyard.len(),"lands_in_play":lands.len(),
        "tapped_lands":lands.iter().filter(|id|game.is_tapped(**id)).count(),"resolved_stack_entries":resolved}),
    )
}

fn record(
    rows: &mut Vec<Value>,
    card: &str,
    scenario: Value,
    expected: Value,
    result: Result<Value, String>,
    scope: &str,
    checksum: &str,
) {
    let (status, actual) = match result {
        Ok(actual) if actual == expected => ("expected_result_observed", actual),
        Ok(actual) => ("semantic_mismatch", actual),
        Err(error) if error.starts_with("Resolution failed:") => {
            ("resolution_failed", json!({"error": error}))
        }
        Err(error) => ("execution_or_fixture_error", json!({"error": error})),
    };
    rows.push(json!({"card":card, "scenario":scenario, "status":status,
        "expected":expected, "actual":actual, "scope":scope, "artifact_checksum":checksum, "seed":SEED}));
    if let Some(category) = match status {
        "semantic_mismatch" => Some("silent_wrong_result"),
        "resolution_failed" => Some("runtime_exception"),
        _ => None,
    } {
        rows.last_mut().unwrap()["outcome_category"] = json!(category);
    }
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_canonical_cards_implicated_by_engine_failures() {
    let names = [
        "Ninja of the Deep Hours",
        "Runadi, Behemoth Caller",
        "Communal Brewing",
        "Wildgrowth Archaic",
        "Exquisite Archangel",
        "Aclazotz, Deepest Betrayal",
        "Alpine Guide",
        "Braids, Arisen Nightmare",
        "Choco, Seeker of Paradise",
        "Mountain",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let definitions: HashMap<_, _> = payloads
        .into_values()
        .flatten()
        .map(|payload| {
            let builder = ironsmith_compiler::CardDefinitionBuilder::new(
                CardId::new(),
                payload.parse_name.as_deref().unwrap_or(&payload.name),
            );
            let (artifact, definition) = ironsmith_registry::compile_builder_to_artifact(
                builder,
                &payload.parse_input,
                false,
            )
            .unwrap();
            (payload.name, (definition, artifact.payload_checksum))
        })
        .collect();
    let mut rows = Vec::new();
    let name = "Ninja of the Deep Hours";
    let (definition, checksum) = &definitions[name];
    record(
        &mut rows,
        name,
        json!({"unblocked_attacker":1,"defender":"Bob"}),
        json!({
            "returned_to_hand":true,
        "returned_object_still_attacks_after_payment":false, "attackers_after_resolution":1,
        "ninja_entered_tapped_and_attacking_defender":true}),
        ninjutsu(definition),
        "canonical legal ninjutsu activation, cost payment, actual stack resolution and CombatState",
        checksum,
    );
    for (name, mana_value, ingredients, colors, expected) in [
        (
            "Runadi, Behemoth Caller",
            4,
            0,
            vec![ManaSymbol::Colorless],
            0,
        ),
        (
            "Runadi, Behemoth Caller",
            6,
            0,
            vec![ManaSymbol::Colorless],
            2,
        ),
        (
            "Runadi, Behemoth Caller",
            9,
            0,
            vec![ManaSymbol::Colorless],
            5,
        ),
        ("Communal Brewing", 6, 0, vec![ManaSymbol::Colorless], 0),
        ("Communal Brewing", 6, 2, vec![ManaSymbol::Colorless], 2),
        ("Communal Brewing", 6, 5, vec![ManaSymbol::Colorless], 5),
        ("Wildgrowth Archaic", 6, 0, vec![ManaSymbol::White], 1),
        (
            "Wildgrowth Archaic",
            6,
            0,
            vec![ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black],
            3,
        ),
    ] {
        let (definition, checksum) = &definitions[name];
        record(
            &mut rows,
            name,
            json!({"mana_value":mana_value,"ingredients":ingredients,"mana_colors":format!("{colors:?}")}),
            json!({"plus_one_counters":expected,"resolved_stack_entries":if name == "Runadi, Behemoth Caller" && mana_value < 5 {1} else {2}}),
            grant_entry_counters(definition, mana_value, ingredients, &colors),
            "canonical source on battlefield; legal creature cast with exact mana pool; cast event, grant trigger and permanent spell resolve normally; preexisting ingredient counters seeded",
            checksum,
        );
    }
    let name = "Exquisite Archangel";
    let (definition, checksum) = &definitions[name];
    for (lethal, option) in [(false, 0), (true, 0), (true, 1)] {
        let exile = usize::from(!lethal || option == 0);
        record(
            &mut rows,
            name,
            json!({"player_life_before_sba":0,"angel_damage":if lethal {5} else {0},"destination_option":option}),
            json!({"player_in_game":true,"life":20,"angel_in_exile":exile,"angel_in_graveyard":1-exile}),
            simultaneous_loss(definition, lethal, option),
            "canonical static loss replacement; simultaneous lethal creature damage/player zero life seeded; actual state-based action application",
            checksum,
        );
    }
    let name = "Aclazotz, Deepest Betrayal";
    let (definition, checksum) = &definitions[name];
    for hands in [[0, 0], [1, 0], [1, 1]] {
        record(
            &mut rows,
            name,
            json!({"opponent_hands":hands}),
            json!({"drawn":hands.iter().filter(|n|**n==0).count(),"opponent_hands":[0,0],
                "opponent_graveyards":hands,"resolved_stack_entries":1}),
            aclazotz(definition, hands),
            "actual legal attack declaration and queued attack trigger; nonland opponent hands; all source abilities retained",
            checksum,
        );
    }
    let name = "Alpine Guide";
    let (definition, checksum) = &definitions[name];
    for destination in [Zone::Graveyard, Zone::Exile] {
        record(
            &mut rows,
            name,
            json!({"source_destination":format!("{destination:?}")}),
            json!({"mountains_in_play":0,"mountains_in_graveyard":1,"resolved_stack_entries":1}),
            alpine(definition, &definitions["Mountain"].0, destination),
            "actual source battlefield departure with a legal controlled Mountain to sacrifice; event drain and stack resolution",
            checksum,
        );
    }
    let name = "Braids, Arisen Nightmare";
    let (definition, checksum) = &definitions[name];
    for (controller, bob) in [(false, false), (true, false), (true, true)] {
        record(
            &mut rows,
            name,
            json!({"controller_sacrifices":controller,"bob_sacrifices":bob,"cara_has_no_matching_permanent":true}),
            json!({"drawn":if controller {2-usize::from(bob)} else {0},
                "opponent_life":[if controller && !bob {18} else {20},if controller {18} else {20}],
                "graveyards":[usize::from(controller),usize::from(controller&&bob),0],"resolved_stack_entries":1}),
            braids(definition, controller, bob),
            "engine end-step event producer; explicit controller/opponent optional choices and legal artifact sacrifices; queue and stack resolution",
            checksum,
        );
    }
    let name = "Choco, Seeker of Paradise";
    let (definition, checksum) = &definitions[name];
    record(
        &mut rows,
        name,
        json!({"attacking_birds":2,"library_top_to_bottom":["Bird attacker fixture","Mountain"]}),
        json!({"hand":1,"library":0,"graveyard":0,"lands_in_play":1,"tapped_lands":1,"resolved_stack_entries":2}),
        choco(definition, &definitions["Mountain"].0),
        "actual two-Bird attack declaration; select first looked-at card for hand and remaining land for battlefield; follow-up landfall drained",
        checksum,
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
            .stdout
    };
    let head = String::from_utf8_lossy(&git(&["rev-parse", "HEAD"]))
        .trim()
        .to_string();
    let diff_hash = Sha256::digest(git(&["diff", "--binary", "HEAD"]))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let report = json!({"scope":"21 expected-result scenarios on nine canonical cards implicated by unit failures or synthetic audit exceptions; not corpus completeness",
        "provenance":{"git_head":head,"tracked_worktree_diff_sha256":diff_hash,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","cards":"cards.json"},
        "synthetic_fixture_caveats":[{
            "family":"aggregate attack-trigger amount",
            "classification":"fixture_missing_attack_declaration_metadata",
            "example_card":"Choco, Seeker of Paradise",
            "synthetic_scenario":"attack/source",
            "synthetic_error":"EventValue(Amount) requires a numeric triggering event",
            "detail":"CreatureAttackedEvent::new supplies no declared_attackers. Real attack declarations attach the complete attacking group, which AttacksTrigger uses to bind event_value_amount. A synthetic numeric-context failure alone does not confirm this family is broken. The actual producer-event outcome is independently recorded in rows."
        }], "rows":rows});
    let out = std::env::var_os("IR_RUNTIME_ENGINE_REPORT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("reports/runtime-audit/engine-card-reproductions.json"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Engine card reproduction report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

fn delayed_sacrifice_after_controller_change(
    definition: &CardDefinition,
    transfer_control: bool,
) -> Result<Value, String> {
    let mut game = setup();
    game.turn.turn_number = 3;
    let bob = PlayerId::from_index(1);
    let buried = game.create_object_from_definition(
        &creature("Delayed-sacrifice creature fixture", 3),
        bob,
        Zone::Graveyard,
    );
    let creature_stable = game.object(buried).unwrap().stable_id;
    let hand = game.create_object_from_definition(definition, alice(), Zone::Hand);
    let aura_stable = game.object(hand).unwrap().stable_id;
    for symbol in [ManaSymbol::Black, ManaSymbol::Colorless] {
        game.player_mut(alice()).unwrap().mana_pool.add(symbol, 10);
    }
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand))
        .ok_or("canonical reanimation spell has no legal cast")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    );
    for _ in 0..24 {
        if !game.stack.is_empty() || progress.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = progress else {
            break;
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        );
    }
    progress.map_err(|e| format!("announcement: {e}"))?;
    if game.stack.len() != 1 {
        return Err(format!(
            "expected one announced spell, got {}",
            game.stack.len()
        ));
    }
    resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| format!("spell: {e}"))?;
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
        .map_err(|e| format!("entry queue: {e}"))?;
    if game.stack.len() != 1 {
        return Err(format!(
            "expected one reanimation trigger, got {}",
            game.stack.len()
        ));
    }
    resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| format!("entry trigger: {e}"))?;
    let returned = game
        .find_object_by_stable_id(creature_stable)
        .ok_or("lost creature")?;
    let aura = game
        .find_object_by_stable_id(aura_stable)
        .ok_or("lost source")?;
    if game.object(returned).unwrap().zone != Zone::Battlefield
        || game.controller_of_id(returned) != Some(alice())
        || game.object(aura).unwrap().attached_to
            != Some(ironsmith::object::AttachmentTarget::Object(returned))
    {
        return Err("reanimation/attachment prerequisite was not established".into());
    }
    if transfer_control {
        game.set_current_controller(returned, bob);
    }
    game.move_object_by_effect(aura, Zone::Graveyard)
        .ok_or("source failed to leave battlefield")?;
    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
        .map_err(|e| format!("departure queue: {e}"))?;
    if game.stack.len() != 1 {
        return Err(format!(
            "expected one delayed sacrifice trigger, got {}",
            game.stack.len()
        ));
    }
    resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| format!("delayed sacrifice: {e}"))?;
    let now = game
        .find_object_by_stable_id(creature_stable)
        .ok_or("lost creature after departure")?;
    Ok(json!({
        "reanimation_and_attachment_verified": true,
        "creature_controller_before_departure": if transfer_control {"Bob"} else {"Alice"},
        "creature_zone_after_delayed_trigger": format!("{:?}", game.object(now).unwrap().zone),
        "resolved_stack_entries": 3,
    }))
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_canonical_delayed_sacrifice_controller_family() {
    let names = ["Animate Dead", "Dance of the Dead", "Necromancy"]
        .map(str::to_owned)
        .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut rows = Vec::new();
    for name in names {
        let payload = &payloads[&name][0];
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        let (artifact, definition) =
            ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
                .unwrap();
        for transfer_control in [false, true] {
            record(
                &mut rows,
                &name,
                json!({"creature_owner":"Bob", "aura_controller":"Alice", "transfer_reanimated_creature_to_bob":transfer_control}),
                json!({"reanimation_and_attachment_verified":true,
                    "creature_controller_before_departure":if transfer_control {"Bob"} else {"Alice"},
                    "creature_zone_after_delayed_trigger":"Graveyard", "resolved_stack_entries":3}),
                delayed_sacrifice_after_controller_change(&definition, transfer_control),
                "canonical legal spell cast on opponent's graveyard creature, actual ETB/reanimation/attachment, optional direct control change, real Aura departure and delayed stack trigger; unique CardIds",
                &artifact.payload_checksum,
            );
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let binary_sha = Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    let report = json!({
        "scope":"Three canonical reanimation Auras; source leaves while creature stays under original controller or changes to its owner; expected-result scenarios, not whole-card semantics",
        "provenance":{"git_head":String::from_utf8_lossy(&head.stdout).trim(),"binary":binary,"binary_sha256":binary_sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Controller transfer is seeded directly. Legal spell cast, targets, mana payment, actual reanimation, attachment, Aura departure and delayed trigger resolution are exercised. No claim about all possible zone/control changes or cleanup timing.",
        "rows":rows,
    });
    let out = root.join("reports/runtime-audit/delayed-sacrifice-family.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Delayed sacrifice family report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct MaximumGraveyardTargets {
    requested: usize,
    offered: Vec<Value>,
}
impl DecisionMaker for MaximumGraveyardTargets {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            self.offered.push(json!({"min":requirement.min_targets,
                "max":requirement.max_targets,"legal_count":requirement.legal_targets.len()}));
            let count = requirement
                .max_targets
                .unwrap_or(self.requested)
                .min(self.requested)
                .min(requirement.legal_targets.len());
            selected.extend(requirement.legal_targets.iter().take(count).copied());
        }
        selected
    }
}

fn glissa_poison_count(
    definition: &CardDefinition,
    opponent_poison: [u32; 2],
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    // The controller's poison must not count as an opponent with poison.
    game.player_mut(alice()).unwrap().poison_counters = 3;
    for (i, poison) in opponent_poison.into_iter().enumerate() {
        game.player_mut(PlayerId::from_index(i as u8 + 1))
            .unwrap()
            .poison_counters = poison;
    }
    let filler = creature("Graveyard return candidate", 2);
    for _ in 0..3 {
        game.create_object_from_definition(&filler, alice(), Zone::Graveyard);
    }
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let stable = game.object(source).unwrap().stable_id;
    game.move_object_by_effect(source, Zone::Graveyard)
        .ok_or("source death failed")?;
    let mut dm = MaximumGraveyardTargets {
        requested: opponent_poison.into_iter().filter(|n| *n >= 3).count(),
        offered: Vec::new(),
    };
    let result = resolve_queue(&mut game, &mut TriggerQueue::new(), &mut dm);
    *trace = dm.offered;
    let resolved = result?;
    trace.push(json!({"resolved_stack_entries":resolved}));
    let source = game
        .find_object_by_stable_id(stable)
        .ok_or("source lost after death")?;
    Ok(
        json!({"source_exiled":game.object(source).unwrap().zone == Zone::Exile,
        "returned_cards":game.player(alice()).unwrap().hand.len(),
        "remaining_graveyard_cards":game.player(alice()).unwrap().graveyard.len()}),
    )
}

fn seifer_blocker_threshold(
    definition: &CardDefinition,
    blocker_count: usize,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    // A third player's creature can trigger Seifer when it attacks another
    // opponent. This avoids the independent "you attack" goad trigger.
    game.turn.active_player = cara;
    game.turn.priority_player = Some(cara);
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let creature = creature("Combat threshold fixture", 2);
    let attacker = game.create_object_from_definition(&creature, cara, Zone::Battlefield);
    let other_attacker = game.create_object_from_definition(&creature, cara, Zone::Battlefield);
    let idle = game.create_object_from_definition(&creature, cara, Zone::Battlefield);
    let blockers = (0..blocker_count)
        .map(|_| game.create_object_from_definition(&creature, bob, Zone::Battlefield))
        .collect::<Vec<_>>();
    let mut queue = attack(&mut game, &[attacker, other_attacker])?;
    let attack_triggers = resolve_queue(&mut game, &mut queue, &mut SelectFirstDecisionMaker)?;
    if attack_triggers != 0 {
        return Err(format!(
            "unrelated attack trigger in fixture: {attack_triggers}"
        ));
    }
    game.turn.step = Some(Step::DeclareBlockers);
    let mut combat = game.combat.clone().unwrap();
    let declarations = blockers
        .iter()
        .map(|id| ironsmith::decision::BlockerDeclaration {
            blocker: *id,
            blocking: attacker,
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &declarations,
        bob,
    )
    .map_err(|e| e.to_string())?;
    game.combat = Some(combat);
    let resolved = resolve_queue(&mut game, &mut queue, &mut SelectFirstDecisionMaker)?;
    let has_deathtouch = |id| {
        game.object_has_static_ability_id(
            id,
            ironsmith::static_abilities::StaticAbilityId::Deathtouch,
        )
    };
    Ok(
        json!({"triggering_attacker_deathtouch":has_deathtouch(attacker),
        "other_attacker_deathtouch":has_deathtouch(other_attacker),
        "nonattacking_creature_deathtouch":has_deathtouch(idle),
        "seifer_deathtouch":has_deathtouch(source),
        "blockers_with_deathtouch":blockers.iter().filter(|id|has_deathtouch(**id)).count(),
        "resolved_block_triggers":resolved}),
    )
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_canonical_glissa_and_seifer_thresholds() {
    let names = ["Glissa's Retriever", "Seifer, Balamb Rival"]
        .map(str::to_owned)
        .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut rows = Vec::new();
    for name in names {
        let payload = &payloads[&name][0];
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        let (artifact, definition) =
            ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
                .unwrap();
        if name == "Glissa's Retriever" {
            for poison in [[0, 0], [2, 2], [3, 2], [3, 3]] {
                let expected_count = poison.into_iter().filter(|n| *n >= 3).count();
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"controller_poison":3,"opponent_poison":poison,"graveyard_candidates":3,"choose_maximum_allowed_targets":true}),
                    json!({"source_exiled":true,"returned_cards":expected_count,
                        "remaining_graveyard_cards":3-expected_count}),
                    glissa_poison_count(&definition, poison, &mut trace),
                    "canonical source death via actual battlefield-to-graveyard event; death exile and reflexive return trigger; choose greatest target count offered up to oracle X; poison totals seeded",
                    &artifact.payload_checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            for blockers in [0, 1, 2] {
                record(
                    &mut rows,
                    &name,
                    json!({"source_controller":"Alice","active_attacking_player":"Cara","defending_player":"Bob","blockers_on_first_attacker":blockers,"other_unblocked_attackers":1,"idle_creatures":1}),
                    json!({"triggering_attacker_deathtouch":blockers>=2,"other_attacker_deathtouch":false,
                        "nonattacking_creature_deathtouch":false,"seifer_deathtouch":false,
                        "blockers_with_deathtouch":0,"resolved_block_triggers":usize::from(blockers>=2)}),
                    seifer_blocker_threshold(&definition, blockers),
                    "actual legal attack and block declarations, producer events, trigger queue and stack resolution; two opponents of source controller attack/block each other; all source abilities retained",
                    &artifact.payload_checksum,
                );
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let binary_sha = Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    let report = json!({"scope":"Expected-result boundary probes for canonical Glissa's Retriever and Seifer, Balamb Rival; not whole-card semantics",
        "provenance":{"git_head":String::from_utf8_lossy(&head.stdout).trim(),"binary":binary,"binary_sha256":binary_sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Seeded source permanents and poison counters. Combat declarations are checked by the engine. Glissa's death is an effect-driven zone change; casting and lethal damage are outside these probes. Report generation succeeding does not mean the card outcomes passed.",
        "rows":rows});
    let out = root.join("reports/runtime-audit/glissa-seifer-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Threshold reproduction report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct OptionalFixtureChoices {
    accept: bool,
    decisions: Vec<String>,
}
impl DecisionMaker for OptionalFixtureChoices {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.decisions
            .push(format!("{} => {}", ctx.description, self.accept));
        self.accept
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        SelectFirstDecisionMaker.decide_targets(game, ctx)
    }
}

fn announce_chosen(
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
        let ctx = match progress.map_err(|e| e.to_string())? {
            GameProgress::NeedsDecisionCtx(ctx) => ctx,
            other => return Err(format!("announcement fixture cannot continue: {other:?}")),
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, dm,
        );
    }
    Err("announcement exceeded 24 decisions".into())
}

fn cast_from_hand(
    game: &mut GameState,
    definition: &CardDefinition,
    caster: PlayerId,
    dm: &mut impl DecisionMaker,
) -> Result<TriggerQueue, String> {
    game.turn.priority_player = Some(caster);
    let source = game.create_object_from_definition(definition, caster, Zone::Hand);
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(caster).unwrap().mana_pool.add(symbol, 12);
    }
    let action = compute_legal_actions(game, caster)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == source))
        .ok_or_else(|| format!("no legal cast of {}", definition.name()))?;
    announce_chosen(game, action, dm)
}

fn plargg_reveal_stop(
    definition: &CardDefinition,
    prefix: &[&str],
    trace: &mut Vec<String>,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    game.remove_summoning_sickness(source);
    let sentinel = creature("Unrevealed sentinel", 1);
    game.create_object_from_definition(&sentinel, alice(), Zone::Library);
    let cheap = creature("Eligible cheap creature", 2);
    game.create_object_from_definition(&cheap, alice(), Zone::Library);
    for kind in prefix.iter().rev() {
        let card = match *kind {
            "land" => CardDefinitionBuilder::new(CardId::new(), "Ineligible land")
                .card_types(vec![CardType::Land])
                .build(),
            "expensive" => creature("Ineligible expensive creature", 6),
            "legendary" => {
                CardDefinitionBuilder::new(CardId::new(), "Ineligible legendary creature")
                    .card_types(vec![CardType::Creature])
                    .supertypes(vec![ironsmith::Supertype::Legendary])
                    .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build()
            }
            _ => return Err("unknown library fixture kind".into()),
        };
        game.create_object_from_definition(&card, alice(), Zone::Library);
    }
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Red, 5);
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|action| {
            let LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } = action
            else {
                return false;
            };
            *id == source
                && game
                    .current_activated_ability(source, *ability_index)
                    .is_some_and(|ability| {
                        ability.effects.all_effects().into_iter().any(|effect| {
                            effect
                                .downcast_ref::<ironsmith::effects::ConsultTopOfLibraryEffect>()
                                .is_some()
                        })
                    })
        })
        .ok_or("no legal reveal ability activation")?;
    let mut dm = OptionalFixtureChoices {
        accept: false,
        decisions: Vec::new(),
    };
    let mut queue = announce_chosen(&mut game, action, &mut dm)?;
    let result = resolve_queue(&mut game, &mut queue, &mut dm);
    *trace = dm.decisions;
    result?;
    let top = game
        .player(alice())
        .unwrap()
        .library
        .last()
        .copied()
        .and_then(|id| game.object(id))
        .map(|object| object.name.to_string());
    Ok(json!({"library_top_after_declining_cast":top,
        "library_count":game.player(alice()).unwrap().library.len(),
        "source_tapped":game.is_tapped(source),"stack_empty":game.stack.is_empty()}))
}

fn akiri_equipped(
    definition: &CardDefinition,
    accept: bool,
    trace: &mut Vec<String>,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let wearer = game.create_object_from_definition(
        &creature("Equipment wearer", 2),
        alice(),
        Zone::Battlefield,
    );
    let equipment = CardDefinitionBuilder::new(CardId::new(), "Attached Equipment fixture")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment = game.create_object_from_definition(&equipment, alice(), Zone::Battlefield);
    if !game.attach_object_to_target(
        equipment,
        ironsmith::object::AttachmentTarget::Object(wearer),
    ) {
        return Err("failed to establish Equipment attachment".into());
    }
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 1);
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
        .ok_or("equipped Akiri fixture has no legal activation")?;
    let mut dm = OptionalFixtureChoices {
        accept,
        decisions: Vec::new(),
    };
    let mut queue = announce_chosen(&mut game, action, &mut dm)?;
    let result = resolve_queue(&mut game, &mut queue, &mut dm);
    *trace = dm.decisions;
    result?;
    Ok(
        json!({"equipment_attached":game.object(equipment).unwrap().attached_to.is_some(),
        "wearer_tapped":game.is_tapped(wearer),
        "wearer_indestructible":game.object_has_static_ability_id(wearer, ironsmith::static_abilities::StaticAbilityId::Indestructible)}),
    )
}

fn blight_exile_processing(
    definition: &CardDefinition,
    accept: bool,
    trace: &mut Vec<String>,
) -> Result<Value, String> {
    let mut game = setup();
    for seat in [1, 2] {
        game.create_object_from_definition(
            &creature("Opponent-owned exiled card", 2),
            PlayerId::from_index(seat),
            Zone::Exile,
        );
    }
    let mut dm = OptionalFixtureChoices {
        accept,
        decisions: Vec::new(),
    };
    let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
    let result = resolve_queue(&mut game, &mut queue, &mut dm);
    *trace = dm.decisions;
    result?;
    let scions = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.subtypes.contains(&Subtype::Scion))
        })
        .count();
    Ok(
        json!({"exiled_cards":game.exile.len(),"opponent_graveyards":[
        game.player(PlayerId::from_index(1)).unwrap().graveyard.len(),
        game.player(PlayerId::from_index(2)).unwrap().graveyard.len()],"scions":scions,
        "herder_on_battlefield":game.battlefield.iter().any(|id|game.object(*id).is_some_and(|o|o.name==definition.name()))}),
    )
}

fn backdraft_after_sorcery(definition: &CardDefinition, damage: i32) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    let sorcery = CardDefinitionBuilder::new(CardId::new(), "Damage history sorcery fixture")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .with_spell_effect(vec![ironsmith::Effect::deal_damage(
            damage,
            ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(
                alice(),
            )),
        )])
        .build();
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = cast_from_hand(&mut game, &sorcery, bob, &mut dm)?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    if game.player(alice()).unwrap().life != 20 - damage {
        return Err("damage sorcery did not establish required actual damage history".into());
    }
    let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    Ok(
        json!({"damaged_player_life":game.player(alice()).unwrap().life,
        "sorcery_caster_life":game.player(bob).unwrap().life}),
    )
}

fn barrins_spite_same_controller(definition: &CardDefinition) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let first = game.create_object_from_definition(
        &creature("Bob first creature", 2),
        bob,
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &creature("Bob second creature", 2),
        bob,
        Zone::Battlefield,
    );
    let stable = [
        game.object(first).unwrap().stable_id,
        game.object(second).unwrap().stable_id,
    ];
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let zones = stable
        .into_iter()
        .map(|id| {
            game.find_object_by_stable_id(id)
                .and_then(|id| game.object(id))
                .map(|o| o.zone)
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"chosen_creatures_on_battlefield":zones.iter().filter(|zone|**zone==Some(Zone::Battlefield)).count(),
        "chosen_creatures_in_graveyard":zones.iter().filter(|zone|**zone==Some(Zone::Graveyard)).count(),
        "chosen_creatures_in_hand":zones.iter().filter(|zone|**zone==Some(Zone::Hand)).count()}),
    )
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_rich_legal_candidate_scenarios() {
    let names = [
        "Plargg, Dean of Chaos // Augusta, Dean of Order",
        "Akiri, Fearless Voyager",
        "Blight Herder",
        "Backdraft",
        "Barrin's Spite",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut rows = Vec::new();
    for name in names {
        let payload = &payloads[&name][0];
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        let (artifact, definition) =
            ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
                .unwrap();
        let checksum = &artifact.payload_checksum;
        if name.starts_with("Plargg") {
            for prefix in [
                vec![],
                vec!["land"],
                vec!["expensive"],
                vec!["land", "expensive"],
                vec!["legendary"],
            ] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"top_prefix":prefix,"then":"eligible MV2 creature","unrevealed_next":"sentinel","decline_optional_cast":true}),
                    json!({"library_top_after_declining_cast":"Unrevealed sentinel","library_count":prefix.len()+2,"source_tapped":true,"stack_empty":true}),
                    plargg_reveal_stop(&definition, &prefix, &mut trace),
                    "legal typed reveal ability activation and paid costs; known library order; decline optional cast so remaining library top independently exposes reveal-stop boundary",
                    checksum,
                );
                rows.last_mut().unwrap()["optional_choice_trace"] = json!(trace);
            }
        } else if name == "Akiri, Fearless Voyager" {
            for accept in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"attached_equipment":true,"optional_unattach":accept}),
                    json!({"equipment_attached":!accept,"wearer_tapped":accept,"wearer_indestructible":accept}),
                    akiri_equipped(&definition, accept, &mut trace),
                    "actual legal W activation with an attached noncreature Equipment and controlled creature; explicitly accept or decline optional unattach",
                    checksum,
                );
                rows.last_mut().unwrap()["optional_choice_trace"] = json!(trace);
            }
        } else if name == "Blight Herder" {
            for accept in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"opponent_owned_exile_cards":2,"optional_process":accept}),
                    json!({"exiled_cards":if accept {0} else {2},"opponent_graveyards":if accept {[1,1]} else {[0,0]},"scions":if accept {3} else {0},"herder_on_battlefield":true}),
                    blight_exile_processing(&definition, accept, &mut trace),
                    "actual legal cast with two opponent-owned cards in exile; explicit optional processing choice; cast trigger and source permanent resolve normally",
                    checksum,
                );
                rows.last_mut().unwrap()["optional_choice_trace"] = json!(trace);
            }
        } else if name == "Backdraft" {
            for damage in [1, 5, 6] {
                record(
                    &mut rows,
                    &name,
                    json!({"prior_sorcery_caster":"Bob","actual_damage_to_alice":damage,"same_turn":true}),
                    json!({"damaged_player_life":20-damage,"sorcery_caster_life":20-damage/2}),
                    backdraft_after_sorcery(&definition, damage),
                    "Bob legally casts and resolves a generic damage sorcery; Alice legally casts canonical Backdraft in same main phase after actual damage history is established",
                    checksum,
                );
            }
        } else {
            record(
                &mut rows,
                &name,
                json!({"legal_creature_targets":2,"both_owned_and_controlled_by":"Bob"}),
                json!({"chosen_creatures_on_battlefield":0,"chosen_creatures_in_graveyard":1,"chosen_creatures_in_hand":1}),
                barrins_spite_same_controller(&definition),
                "actual legal canonical cast selecting two creatures controlled by the same opponent; controller chooses sacrifice and the other returns",
                checksum,
            );
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let mut reader = std::fs::File::open(&binary).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let read = std::io::Read::read(&mut reader, &mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    let binary_sha = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    let report = json!({"scope":"Thirteen expected-result scenarios on five canonical candidate cards using richer prerequisites and actual legal actions",
        "provenance":{"git_head":String::from_utf8_lossy(&head.stdout).trim(),"binary":binary,"binary_sha256":binary_sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Fixtures seed ordinary permanents/library contents/attachments. Backdraft's preceding sorcery is a generic engine definition, actually cast and resolved. Plargg's optional cast is deliberately declined; acceptance is not covered. All observations are scenario-specific; successful report generation is not a card semantics pass.","rows":rows});
    let out = root.join("reports/runtime-audit/rich-legal-candidate-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Rich legal candidate report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

fn tatsumasa_delayed_return(
    definition: &CardDefinition,
    watched_dies: bool,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let stable = game.object(source).unwrap().stable_id;
    game.player_mut(alice())
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 6);
    let action = compute_legal_actions(&game, alice())
        .into_iter()
        .find(|action| {
            let LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } = action
            else {
                return false;
            };
            *id == source
                && game
                    .current_activated_ability(source, *ability_index)
                    .is_some_and(|ability| {
                        format!("{:?}", ability.effects).contains("CreateTokenEffect")
                    })
        })
        .ok_or("no legal token-creation activation")?;
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = announce_chosen(&mut game, action, &mut dm)?;
    let after_cost = game
        .find_object_by_stable_id(stable)
        .ok_or("lost source after exile cost")?;
    if game.object(after_cost).unwrap().zone != Zone::Exile
        || game.player(alice()).unwrap().mana_pool.total() != 0
    {
        return Err("legal activation did not pay six mana and exile source".into());
    }
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let watched = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id).is_some_and(|object| {
                matches!(object.kind, ironsmith::object::ObjectKind::Token)
                    && object.subtypes.contains(&Subtype::Dragon)
                    && object.subtypes.contains(&Subtype::Spirit)
            })
        })
        .ok_or("no created Dragon Spirit token")?;
    if game.calculated_power(watched) != Some(5) || game.calculated_toughness(watched) != Some(5) {
        return Err("created token does not have prerequisite 5/5 characteristics".into());
    }
    let unrelated = CardDefinitionBuilder::new(CardId::new(), "Unrelated Spirit token")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .token()
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let unrelated = game.create_object_from_definition(&unrelated, alice(), Zone::Battlefield);
    game.move_object_by_effect(
        if watched_dies { watched } else { unrelated },
        Zone::Graveyard,
    )
    .ok_or("token did not die")?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let now = game
        .find_object_by_stable_id(stable)
        .ok_or("lost source after token death")?;
    Ok(
        json!({"activation_cost_paid":true,"created_dragon_spirit_verified":true,
        "source_zone_after_token_death":format!("{:?}",game.object(now).unwrap().zone)}),
    )
}

fn portcullis_delayed_return(
    definition: &CardDefinition,
    other_creatures: usize,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    for _ in 0..other_creatures {
        game.create_object_from_definition(
            &creature("Preexisting creature", 2),
            alice(),
            Zone::Battlefield,
        );
    }
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    let entering = creature("Portcullis entering creature", 2);
    let mut dm = SelectFirstDecisionMaker;
    let mut queue = cast_from_hand(&mut game, &entering, bob, &mut dm)?;
    let entry_id = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .map(|entry| entry.object_id)
        .ok_or("incoming creature was not cast onto stack")?;
    let stable = game
        .object(entry_id)
        .ok_or("incoming spell object missing")?
        .stable_id;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let before = game
        .find_object_by_stable_id(stable)
        .ok_or("incoming creature disappeared")?;
    let zone_before = format!("{:?}", game.object(before).unwrap().zone);
    game.move_object_by_effect(source, Zone::Graveyard)
        .ok_or("Portcullis departure failed")?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let after = game
        .find_object_by_stable_id(stable)
        .ok_or("incoming creature lost after departure")?;
    Ok(json!({"creature_zone_before_source_departure":zone_before,
        "creature_zone_after_source_departure":format!("{:?}",game.object(after).unwrap().zone),
        "creature_owner_is_bob":game.object(after).unwrap().owner==bob,
        "creature_controller_is_bob":game.controller_of_id(after)==Some(bob)}))
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_canonical_delayed_return_candidates() {
    let names = ["Tatsumasa, the Dragon's Fang", "Portcullis"]
        .map(str::to_owned)
        .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let mut rows = Vec::new();
    for name in names {
        let payload = &payloads[&name][0];
        let builder = ironsmith_compiler::CardDefinitionBuilder::new(
            CardId::new(),
            payload.parse_name.as_deref().unwrap_or(&payload.name),
        );
        let (artifact, definition) =
            ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
                .unwrap();
        if name.starts_with("Tatsumasa") {
            for watched in [false, true] {
                record(
                    &mut rows,
                    &name,
                    json!({"dead_token":if watched {"created Dragon Spirit"} else {"unrelated Spirit"}}),
                    json!({"activation_cost_paid":true,"created_dragon_spirit_verified":true,
                        "source_zone_after_token_death":if watched {"Battlefield"} else {"Exile"}}),
                    tatsumasa_delayed_return(&definition, watched),
                    "actual legal activation, six mana paid and source exiled as cost, real stack resolution creates Dragon Spirit, actual token death producer event and delayed trigger drain",
                    &artifact.payload_checksum,
                );
            }
        } else {
            for others in [0, 1, 2] {
                record(
                    &mut rows,
                    &name,
                    json!({"other_creatures_before_entry":others,"entering_creature_owner_and_controller":"Bob"}),
                    json!({"creature_zone_before_source_departure":if others>=2 {"Exile"} else {"Battlefield"},
                        "creature_zone_after_source_departure":"Battlefield","creature_owner_is_bob":true,"creature_controller_is_bob":true}),
                    portcullis_delayed_return(&definition, others),
                    "canonical Portcullis on battlefield; generic creature legally cast with actual ETB and exile trigger; actual source departure and delayed return trigger drain",
                    &artifact.payload_checksum,
                );
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let binary = std::env::current_exe().unwrap();
    let mut reader = std::fs::File::open(&binary).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let size = std::io::Read::read(&mut reader, &mut buffer).unwrap();
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    let sha = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .unwrap();
    let report = json!({"scope":"Five canonical expected-result scenarios for delayed-return candidates from synthetic native failures",
        "provenance":{"git_head":String::from_utf8_lossy(&head.stdout).trim(),"binary":binary,"binary_sha256":sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Sources and ordinary fixture permanents are seeded. Tatsumasa activation/costs and Portcullis's incoming creature cast are legal engine actions. Death/departure is performed by ordinary zone-change effects; lethal combat or removal-spell announcement is not exercised. Passing report generation does not mean every observed card outcome passed.","rows":rows});
    let out = root.join("reports/runtime-audit/delayed-return-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Canonical delayed return report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

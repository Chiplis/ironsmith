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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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
    if !apply_state_based_actions_with(&mut game, &mut dm)
        .map_err(|error| format!("state-based action execution failed: {error}"))? {
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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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
        game.set_current_controller(returned, bob).expect("finite controller fixture must refresh successfully");
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
    let action = compute_legal_actions(game, caster).expect("fixture has complete replacement state")
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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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
    let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
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

fn bat_duration_return(definition: &CardDefinition, accept: bool) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let victim =
        game.create_object_from_definition(&creature("Bob hand creature", 2), bob, Zone::Hand);
    let stable = game.object(victim).unwrap().stable_id;
    let mut dm = OptionalFixtureChoices {
        accept,
        decisions: Vec::new(),
    };
    let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
    resolve_queue(&mut game, &mut queue, &mut dm)?;
    let victim = game
        .find_object_by_stable_id(stable)
        .ok_or("selected card missing")?;
    let zone_before = format!("{:?}", game.object(victim).unwrap().zone);
    let source = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == definition.name())
        })
        .ok_or("Bat did not enter battlefield")?;
    game.move_object_by_effect(source, Zone::Graveyard)
        .ok_or("Bat departure failed")?;
    // Exile-until returns can require entry choices. The current priority
    // engine processes those pending returns before offering legal actions.
    // Merely draining trigger events does not run that path.
    ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm)
        .map_err(|error| error.to_string())?;
    let returned = game
        .find_object_by_stable_id(stable)
        .ok_or("selected card disappeared")?;
    Ok(json!({"selected_card_zone_after_etb":zone_before,
        "selected_card_zone_after_source_leaves_and_priority_advances":format!("{:?}",game.object(returned).unwrap().zone),
        "opponent_hand_count":game.player(bob).unwrap().hand.len()}))
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_canonical_bat_duration_return() {
    let name = "Deep-Cavern Bat";
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(
        CardId::new(),
        payload.parse_name.as_deref().unwrap_or(&payload.name),
    );
    let (artifact, definition) =
        ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
            .unwrap();
    let mut rows = Vec::new();
    for accept in [false, true] {
        record(
            &mut rows,
            name,
            json!({"accept_exile":accept,"targeted_opponent_nonland_hand_cards":1}),
            json!({"selected_card_zone_after_etb":if accept {"Exile"} else {"Hand"},
                "selected_card_zone_after_source_leaves_and_priority_advances":"Hand","opponent_hand_count":1}),
            bat_duration_return(&definition, accept),
            "actual legal canonical cast and targeted ETB; choose accept/decline exile; actual source departure followed by normal priority advancement to process pending duration-end returns",
            &artifact.payload_checksum,
        );
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
    let report = json!({"scope":"Two canonical Deep-Cavern Bat scenarios disambiguating direct-effect native-test failures from complete priority processing",
        "provenance":{"binary":binary,"binary_sha256":sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"No opponent response invalidates target or source before ETB. Only return to the same opponent's hand with the source leaving after ETB is exercised. Native direct-effect tests omit the priority step that processes deferred duration-end returns.","rows":rows});
    let out = root.join("reports/runtime-audit/deep-cavern-bat-reproduction.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Bat priority reproduction: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct XFixtureChoices {
    x: u32,
    desired_targets: Vec<ironsmith::game_state::Target>,
    zero_targets: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for XFixtureChoices {
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let chosen = self.x.clamp(ctx.min, ctx.max);
        self.trace.push(json!({"choice":"number","description":ctx.description,"min":ctx.min,"max":ctx.max,"chosen":chosen,"is_x":ctx.is_x_value}));
        chosen
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        if self.zero_targets && self.x == 0 && ctx.requirements.iter().all(|r| r.min_targets == 0) {
            self.trace.push(json!({"choice":"targets","selected":0}));
            return Vec::new();
        }
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let desired = self
                .desired_targets
                .iter()
                .copied()
                .filter(|target| requirement.legal_targets.contains(target))
                .take(
                    requirement
                        .max_targets
                        .unwrap_or(self.desired_targets.len()),
                )
                .collect::<Vec<_>>();
            if desired.len() >= requirement.min_targets {
                selected.extend(desired);
            } else {
                return SelectFirstDecisionMaker.decide_targets(game, ctx);
            }
        }
        self.trace
            .push(json!({"choice":"targets","selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let desired = self
            .desired_targets
            .iter()
            .filter_map(|target| match target {
                ironsmith::game_state::Target::Object(id)
                    if ctx.candidates.iter().any(|c| c.legal && c.id == *id) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .take(ctx.max.unwrap_or(1))
            .collect::<Vec<_>>();
        if !desired.is_empty() && desired.len() >= ctx.min {
            desired
        } else {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
    }
    fn decide_distribute(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::DistributeContext,
    ) -> Vec<(ironsmith::game_state::Target, u32)> {
        self.trace
            .push(json!({"choice":"distribute","total":ctx.total,"targets":ctx.targets.len()}));
        if ctx.total == 0 || ctx.targets.is_empty() {
            return Vec::new();
        }
        if ctx.targets.len() > 1 && ctx.total >= 2 {
            vec![
                (ctx.targets[0].target, ctx.total - 1),
                (ctx.targets[1].target, 1),
            ]
        } else {
            vec![(ctx.targets[0].target, ctx.total)]
        }
    }
}

fn finish_with_priority(
    game: &mut GameState,
    queue: &mut TriggerQueue,
    dm: &mut impl DecisionMaker,
) -> Result<usize, String> {
    let mut resolved = 0;
    for _ in 0..32 {
        ironsmith::game_loop::advance_priority_with_dm(game, queue, dm)
            .map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
        resolved += 1;
    }
    Err("fixture exceeded32 priority/stack steps".into())
}

fn x_etb_card(
    definition: &CardDefinition,
    x: u32,
    cast: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let first = game.create_object_from_definition(
        &creature("First counter/attachment target", 2),
        alice(),
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &creature("Second counter target", 2),
        alice(),
        Zone::Battlefield,
    );
    let is_brood = definition.name() == "Broodlord";
    let desired_targets = if definition.name() == "Ajani's Anguish" {
        vec![ironsmith::game_state::Target::Player(PlayerId::from_index(
            1,
        ))]
    } else if is_brood {
        vec![
            ironsmith::game_state::Target::Object(first),
            ironsmith::game_state::Target::Object(second),
        ]
    } else {
        vec![ironsmith::game_state::Target::Object(first)]
    };
    let mut dm = XFixtureChoices {
        x,
        desired_targets,
        zero_targets: is_brood,
        trace: Vec::new(),
    };
    let mut queue;
    let source_stable;
    let paid;
    if cast {
        queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        paid = 72 - game.player(alice()).unwrap().mana_pool.total();
        let entry = game
            .stack
            .iter()
            .find(|entry| !entry.is_ability)
            .ok_or("canonical X spell missing from stack")?;
        source_stable = game.object(entry.object_id).unwrap().stable_id;
        trace.push(json!({"stage":"announced","mana_spent":paid,"stack_x_value":entry.x_value}));
        if entry.x_value != Some(x) {
            return Err(format!("requested X={x}, announced {:?}", entry.x_value));
        }
    } else {
        paid = 0;
        let hand = game.create_object_from_definition(definition, alice(), Zone::Hand);
        source_stable = game.object(hand).unwrap().stable_id;
        let producer = CardDefinitionBuilder::new(CardId::new(), "Noncast entry effect fixture")
            .card_types(vec![CardType::Artifact])
            .build();
        let producer = game.create_object_from_definition(&producer, alice(), Zone::Battlefield);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(
            producer,
            alice(),
            vec![ironsmith::Effect::new(
                ironsmith::effects::PutOntoBattlefieldEffect::new(
                    ironsmith::target::ChooseSpec::SpecificObject(hand),
                    false,
                    ironsmith::target::PlayerFilter::You,
                ),
            )],
        ));
        queue = TriggerQueue::new();
        trace.push(json!({"stage":"noncast_entry","method":"ordinary PutOntoBattlefieldEffect with normal Aura attachment choices","mana_spent":0}));
    }
    let result = finish_with_priority(&mut game, &mut queue, &mut dm);
    trace.extend(dm.trace);
    if let Some(source) = game
        .find_object_by_stable_id(source_stable)
        .and_then(|id| game.object(id))
    {
        trace.push(json!({"stage":"source_after_entry_attempt","zone":format!("{:?}",source.zone),
            "source_x_value":source.x_value,"attached_to_first_target":source.attached_to==Some(ironsmith::object::AttachmentTarget::Object(first))}));
    }
    result?;
    let source = game
        .find_object_by_stable_id(source_stable)
        .ok_or("canonical source disappeared")?;
    let observation = match definition.name() {
        "Ajani's Anguish" => {
            json!({"mana_spent":paid,"opponent_life":game.player(PlayerId::from_index(1)).unwrap().life})
        }
        "Arboreal Alliance" => {
            let stats = game
                .battlefield
                .iter()
                .filter(|id| {
                    game.object(**id)
                        .is_some_and(|o| o.subtypes.contains(&Subtype::Treefolk))
                })
                .map(|id| {
                    vec![
                        game.calculated_power(*id).unwrap_or(0),
                        game.calculated_toughness(*id).unwrap_or(0),
                    ]
                })
                .collect::<Vec<_>>();
            json!({"mana_spent":paid,"living_treefolk_stats":stats})
        }
        "Awakened Awareness" => {
            json!({"mana_spent":paid,"aura_attached_to_chosen_creature":game.object(source).unwrap().attached_to==Some(ironsmith::object::AttachmentTarget::Object(first)),
            "enchanted_counters":game.counter_count(first,CounterType::PlusOnePlusOne),"enchanted_power":game.calculated_power(first),"enchanted_toughness":game.calculated_toughness(first)})
        }
        "Broodlord" => {
            json!({"mana_spent":paid,"source_ravenous_counters":game.counter_count(source,CounterType::PlusOnePlusOne),
            "other_creature_counters":[game.counter_count(first,CounterType::PlusOnePlusOne),game.counter_count(second,CounterType::PlusOnePlusOne)]})
        }
        _ => return Err("unsupported X ETB fixture".into()),
    };
    Ok(observation)
}

fn bane_turn_face_up(
    definition: &CardDefinition,
    x: u32,
    printed_cost: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let witness = CardDefinitionBuilder::new(CardId::new(), "Ten toughness witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(10, 10))
        .build();
    let witness = game.create_object_from_definition(&witness, alice(), Zone::Battlefield);
    let mut dm = XFixtureChoices {
        x,
        desired_targets: Vec::new(),
        zero_targets: false,
        trace: Vec::new(),
    };
    let mut queue;
    let source;
    if printed_cost {
        game.create_object_from_definition(definition, alice(), Zone::Library);
        let producer = game.new_object_id();
        let outcome = ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::new(ironsmith::effects::ManifestTopCardOfLibraryEffect::new(
                ironsmith::target::PlayerFilter::You,
            )),
            &mut ironsmith::effects::EffectContext::new(producer, alice(), &mut dm),
        )
        .map_err(|e| e.to_string())?;
        source = *outcome
            .objects()
            .and_then(|objects| objects.first())
            .ok_or("manifest did not produce face-down object")?;
    } else {
        let hand = game.create_object_from_definition(definition, alice(), Zone::Hand);
        for color in [ManaSymbol::Black, ManaSymbol::Colorless] {
            game.player_mut(alice()).unwrap().mana_pool.add(color, 12);
        }
        let action=compute_legal_actions(&game,alice()).expect("fixture has complete replacement state").into_iter().find(|action|matches!(action,
            LegalAction::CastSpell{spell_id,casting_method:ironsmith::alternative_cast::CastingMethod::FaceDown,..} if *spell_id==hand))
            .ok_or("Bane has no legal face-down cast")?;
        queue = announce_chosen(&mut game, action, &mut dm)?;
        let stable = game
            .object(
                game.stack
                    .iter()
                    .find(|entry| !entry.is_ability)
                    .ok_or("no face-down spell")?
                    .object_id,
            )
            .unwrap()
            .stable_id;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        source = game
            .find_object_by_stable_id(stable)
            .ok_or("cast face-down Bane missing")?;
        trace.push(json!({"stage":"face_down_cast","mana_spent":24-game.player(alice()).unwrap().mana_pool.total()}));
    }
    if !game.is_face_down(source) {
        return Err("Bane prerequisite is not face down".into());
    }
    // Refill the pool and measure the actual turn-face-up payment separately.
    for color in [ManaSymbol::Black, ManaSymbol::Colorless] {
        game.player_mut(alice()).unwrap().mana_pool.add(color, 12);
    }
    let before = game.player(alice()).unwrap().mana_pool.total();
    let method = if printed_cost {
        ironsmith::special_actions::TurnFaceUpMethod::PrintedManaCost
    } else {
        ironsmith::special_actions::TurnFaceUpMethod::TurnFaceUpAbility
    };
    let action=compute_legal_actions(&game,alice()).expect("fixture has complete replacement state").into_iter().find(|action|matches!(action,
        LegalAction::TurnFaceUp{creature_id,method:selected} if *creature_id==source && *selected==method))
        .ok_or("requested Bane turn-face-up method is not legal")?;
    let result = announce_chosen(&mut game, action, &mut dm);
    trace.extend(dm.trace.clone());
    queue = result?;
    let paid = before - game.player(alice()).unwrap().mana_pool.total();
    trace.push(json!({"stage":"turned_face_up","paid":paid,"method":format!("{method:?}")}));
    let result = finish_with_priority(&mut game, &mut queue, &mut dm);
    trace.extend(dm.trace);
    result?;
    Ok(
        json!({"face_up":!game.is_face_down(source),"turn_face_up_mana_spent":paid,
        "witness_power":game.calculated_power(witness),"witness_toughness":game.calculated_toughness(witness)}),
    )
}

fn archdemon_upkeep(definition: &CardDefinition, has_human: bool) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    if has_human {
        let human = CardDefinitionBuilder::new(CardId::new(), "Human to sacrifice")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&human, alice(), Zone::Battlefield);
    }
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    let mut queue = TriggerQueue::new();
    generate_and_queue_step_triggers(&mut game, &mut queue);
    finish_with_priority(&mut game, &mut queue, &mut SelectFirstDecisionMaker)?;
    Ok(
        json!({"controller_life":game.player(alice()).unwrap().life,"source_tapped":game.is_tapped(source),
        "humans_in_graveyard":game.player(alice()).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|o.subtypes.contains(&Subtype::Human))).count()}),
    )
}

fn bristlebud_attack(
    definition: &CardDefinition,
    has_food: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    if has_food {
        let food = CardDefinitionBuilder::new(CardId::new(), "Food fixture")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food])
            .token()
            .build();
        game.create_object_from_definition(&food, alice(), Zone::Battlefield);
    }
    for (name, kind) in [
        ("Bottom sorcery", CardType::Sorcery),
        ("Middle instant", CardType::Instant),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![kind])
            .build();
        game.create_object_from_definition(&definition, alice(), Zone::Library);
    }
    game.create_object_from_definition(&creature("Milled permanent", 2), alice(), Zone::Library);
    let mut queue = attack(&mut game, &[source])?;
    let mut dm = OptionalFixtureChoices {
        accept,
        decisions: Vec::new(),
    };
    let result = finish_with_priority(&mut game, &mut queue, &mut dm);
    trace.push(json!({"optional_choice_callbacks":dm.decisions}));
    result?;
    Ok(
        json!({"foods_on_battlefield":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|o|o.subtypes.contains(&Subtype::Food))).count(),
        "hand":game.player(alice()).unwrap().hand.len(),"library":game.player(alice()).unwrap().library.len(),"graveyard":game.player(alice()).unwrap().graveyard.len()}),
    )
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_actual_x_and_resource_trigger_paths() {
    let names = [
        "Ajani's Anguish",
        "Arboreal Alliance",
        "Awakened Awareness",
        "Bane of the Living",
        "Broodlord",
        "Archdemon of Greed",
        "Bristlebud Farmer",
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
        if [
            "Ajani's Anguish",
            "Arboreal Alliance",
            "Awakened Awareness",
            "Broodlord",
        ]
        .contains(&name.as_str())
        {
            for (x, cast) in [(0, true), (3, true), (0, false)] {
                let paid = if cast {
                    x + match name.as_str() {
                        "Ajani's Anguish" => 1,
                        "Broodlord" => 4,
                        _ => 2,
                    }
                } else {
                    0
                };
                let expected = match name.as_str() {
                    "Ajani's Anguish" => json!({"mana_spent":paid,"opponent_life":20-x}),
                    "Arboreal Alliance" => {
                        json!({"mana_spent":paid,"living_treefolk_stats":if x==0 {vec![]} else {vec![vec![x,x]]}})
                    }
                    "Awakened Awareness" => {
                        json!({"mana_spent":paid,"aura_attached_to_chosen_creature":true,"enchanted_counters":x,"enchanted_power":1+x,"enchanted_toughness":1+x})
                    }
                    _ => {
                        json!({"mana_spent":paid,"source_ravenous_counters":x,"other_creature_counters":if x==0 {[0,0]}else{[x-1,1]}})
                    }
                };
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"x":x,"cast":cast,"two_other_controlled_creatures":true}),
                    expected,
                    x_etb_card(&definition, x, cast, &mut trace),
                    "actual legal paid X cast or ordinary noncast put-onto-battlefield effect; normal target/attachment/distribution choices, producer ETB, priority/SBA and trigger resolution",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Bane of the Living" {
            for (x, printed) in [(0, false), (2, false), (0, true)] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"x":x,"manifest_then_pay_printed_mana":printed}),
                    json!({"face_up":true,"turn_face_up_mana_spent":if printed {4}else{x+2},"witness_power":10-x,"witness_toughness":10-x}),
                    bane_turn_face_up(&definition, x, printed, &mut trace),
                    "legal face-down cast then paid morph X, or ordinary manifest effect then legal printed-mana turn-face-up; actual turned-face-up event and priority/trigger resolution",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                if x > 0
                    && !printed
                    && !trace
                        .iter()
                        .any(|entry| entry["stage"] == "turned_face_up" && entry["paid"] == x + 2)
                {
                    let row = rows.last_mut().unwrap();
                    row["status"] = json!("requested_path_unavailable");
                    row.as_object_mut().unwrap().remove("outcome_category");
                    row["review"] = json!(
                        "Requested positive morph X was not selected or paid. No X choice was offered and onlyBB was paid; observed resolution is another X0 case. Do not claim positive-X morph coverage."
                    );
                }
            }
        } else if name == "Archdemon of Greed" {
            for human in [false, true] {
                record(
                    &mut rows,
                    &name,
                    json!({"human_available":human,"active_player_is_controller":true}),
                    json!({"controller_life":if human {20}else{11},"source_tapped":!human,"humans_in_graveyard":usize::from(human)}),
                    archdemon_upkeep(&definition, human),
                    "canonical back face seeded on battlefield; actual controller upkeep event, optional Human prerequisite, normal priority and trigger resolution; transformation not exercised",
                    checksum,
                );
            }
        } else {
            for (food, accept) in [(false, false), (true, false), (true, true)] {
                let processed = food && accept;
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"food_available":food,"accept_sacrifice_and_return":accept}),
                    json!({"foods_on_battlefield":usize::from(food&&!accept),"hand":usize::from(processed),"library":if processed{0}else{3},"graveyard":if processed{2}else{0}}),
                    bristlebud_attack(&definition, food, accept, &mut trace),
                    "canonical source and optional Food token seeded; actual legal attack declaration; explicit optional sacrifice and permanent-card return, normal priority/SBA and stack processing",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
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
    let report = json!({"scope":"Twenty actual producer-event scenarios on seven canonical X/resource-trigger candidates; zero and positive controls",
        "provenance":{"binary":binary,"binary_sha256":sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Noncast entry and manifest are ordinary generic engine effects, not a specific enabling card. Archdemon's back face and Farmer's Food are seeded; transformation and Farmer ETB are not exercised. Choices are explicit where optional. Report generation is not a semantics pass.","rows":rows});
    let out = root.join("reports/runtime-audit/x-and-resource-trigger-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("X and resource-trigger report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct ReferenceBranchChoices {
    target: ObjectId,
    zero_targets: bool,
    accept: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for ReferenceBranchChoices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        // Do not grow a chain of copies when exercising Chain Stasis's first resolution.
        let answer = self.accept && !ctx.description.to_lowercase().contains("pay");
        self.trace
            .push(json!({"choice":"boolean","description":ctx.description,"chosen":answer}));
        answer
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let target = ironsmith::game_state::Target::Object(self.target);
        let selected = if self.zero_targets && ctx.requirements.iter().all(|r| r.min_targets == 0) {
            Vec::new()
        } else if ctx.requirements.len() == 1 && ctx.requirements[0].legal_targets.contains(&target)
        {
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        };
        self.trace.push(
            json!({"choice":"targets","requested_zero":self.zero_targets,
            "requirements":format!("{:?}",ctx.requirements),"selected":format!("{selected:?}")}),
        );
        selected
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = SelectFirstDecisionMaker.decide_objects(game, ctx);
        self.trace.push(json!({"choice":"objects","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(game, ctx);
        self.trace
            .push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":selected}));
        selected
    }
}

fn optional_reference_target(
    definition: &CardDefinition,
    zero_targets: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let victim = CardDefinitionBuilder::new(CardId::new(), "Legendary reference target")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![ironsmith::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(2, 3))
        .build();
    let target = game.create_object_from_definition(&victim, bob, Zone::Battlefield);
    let stable = game.object(target).unwrap().stable_id;
    let mut dm = ReferenceBranchChoices {
        target,
        zero_targets,
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({
        "target_zone":game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|o|format!("{:?}",o.zone)),
        "alice_life":game.player(alice()).unwrap().life,"bob_life":game.player(bob).unwrap().life,
        "bob_golems":game.battlefield.iter().filter(|id|game.current_controller(**id)==Some(bob)&&game.object(**id).is_some_and(|o|o.subtypes.contains(&Subtype::Golem))).count(),
        "bob_junk":game.battlefield.iter().filter(|id|game.current_controller(**id)==Some(bob)&&game.object(**id).is_some_and(|o|o.subtypes.contains(&Subtype::Junk))).count()
    });
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn chain_stasis_optional(
    definition: &CardDefinition,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let target =
        game.create_object_from_definition(&creature("Chain target", 2), bob, Zone::Battlefield);
    let mut dm = ReferenceBranchChoices {
        target,
        zero_targets: false,
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"target_tapped":game.is_tapped(target),"target_on_battlefield":game.object(target).is_some_and(|o|o.zone==Zone::Battlefield),"bob_mana":game.player(bob).unwrap().mana_pool.total(),"stack_empty":game.stack.is_empty()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn dire_strain_optional(
    definition: &CardDefinition,
    land: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let victim = CardDefinitionBuilder::new(CardId::new(), "Rampage target")
        .card_types(vec![if land {
            CardType::Land
        } else {
            CardType::Artifact
        }])
        .build();
    let target = game.create_object_from_definition(&victim, bob, Zone::Battlefield);
    let stable = game.object(target).unwrap().stable_id;
    for n in 0..3 {
        let basic = CardDefinitionBuilder::new(CardId::new(), &format!("Searchable Forest {n}"))
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Basic])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.create_object_from_definition(&basic, bob, Zone::Library);
    }
    let mut dm = ReferenceBranchChoices {
        target,
        zero_targets: false,
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let lands = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|o| o.name.starts_with("Searchable Forest"))
        })
        .collect::<Vec<_>>();
    let actual = json!({"target_zone":game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|o|format!("{:?}",o.zone)),
        "searched_lands_on_battlefield":lands.len(),"searched_lands_tapped":lands.iter().filter(|id|game.is_tapped(**id)).count(),
        "searched_lands_controlled_by_bob":lands.iter().filter(|id|game.current_controller(**id)==Some(bob)).count(),"bob_library":game.player(bob).unwrap().library.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_optional_reference_context_paths() {
    let names = [
        "Captain Marvel, Shooting Star",
        "Cavalier of Dawn",
        "Commander Sofia Daguerre",
        "Chain Stasis",
        "Dire-Strain Rampage",
    ]
    .map(str::to_owned)
    .to_vec();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
        if name == "Chain Stasis" {
            for accept in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"accept_tap_or_untap":accept,"target_controller_mana":0,"decline_copy_payment":true}),
                    json!({"target_tapped":accept,"target_on_battlefield":true,"bob_mana":0,"stack_empty":true}),
                    chain_stasis_optional(&definition, accept, &mut trace),
                    "Actual legal normal cast targeting opponent's untapped creature; explicit optional tap choice and no mana available for the target controller to copy",
                    &artifact.payload_checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Dire-Strain Rampage" {
            for land in [false, true] {
                for accept in [false, true] {
                    let mut trace = Vec::new();
                    let count = if accept { if land { 2 } else { 1 } } else { 0 };
                    record(
                        &mut rows,
                        &name,
                        json!({"target_is_land":land,"accept_search":accept,"target_controller_library_basic_lands":3}),
                        json!({"target_zone":"Graveyard","searched_lands_on_battlefield":count,"searched_lands_tapped":count,"searched_lands_controlled_by_bob":count,"bob_library":3-count}),
                        dire_strain_optional(&definition, land, accept, &mut trace),
                        "Actual legal normal cast; opposing destructible land or artifact and three basic lands in its controller's library; explicit search acceptance and maximum legal selection; flashback outside scope",
                        &artifact.payload_checksum,
                    );
                    rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                }
            }
        } else {
            for zero in [false, true] {
                let mut trace = Vec::new();
                let captain = name == "Captain Marvel, Shooting Star";
                record(
                    &mut rows,
                    &name,
                    json!({"choose_zero_optional_targets":zero,"available_target":"opponent-owned legendary 2/3 creature"}),
                    json!({"target_zone":if zero {"Battlefield"}else if captain {"Exile"}else{"Graveyard"},
                        "alice_life":if captain&&!zero {22}else{20},"bob_life":if captain&&!zero {22}else{20},
                        "bob_golems":usize::from(name=="Cavalier of Dawn"&&!zero),"bob_junk":usize::from(name=="Commander Sofia Daguerre"&&!zero)}),
                    optional_reference_target(&definition, zero, &mut trace),
                    "Actual legal normal creature cast, producer ETB event and triggered ability, deliberate zero-or-one legal target selection, priority/SBA processing and all resulting triggers",
                    &artifact.payload_checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        }
    }
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
    let report = json!({"scope":"Twelve actual legal-cast scenarios for five optional-action/reference-context candidates with skipped and selected branch controls",
        "provenance":{"binary":binary,"binary_sha256":sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Seeded opposing targets and basic-land libraries; no target becomes illegal while on the stack; Captain Marvel attack and Cavalier death abilities are outside scope. Successful report generation is not a semantics pass.","rows":rows});
    let out = root.join("reports/runtime-audit/optional-reference-context-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Optional reference context report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

struct ControllerReferenceChoices {
    targets: Vec<ironsmith::game_state::Target>,
    accept: bool,
    trace: Vec<Value>,
}
impl DecisionMaker for ControllerReferenceChoices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.trace
            .push(json!({"choice":"boolean","context":format!("{ctx:?}"),"chosen":self.accept}));
        self.accept
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let chosen = self
                .targets
                .iter()
                .copied()
                .filter(|target| {
                    requirement.legal_targets.contains(target) && !selected.contains(target)
                })
                .take(requirement.max_targets.unwrap_or(self.targets.len()))
                .collect::<Vec<_>>();
            if chosen.len() < requirement.min_targets {
                selected = SelectFirstDecisionMaker.decide_targets(game, ctx);
                break;
            }
            selected.extend(chosen);
        }
        self.trace.push(json!({"choice":"targets","requirements":format!("{:?}",ctx.requirements),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let preferred = ctx
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.legal && candidate.name.starts_with("Sacrifice alternative")
            })
            .map(|candidate| candidate.id)
            .take(ctx.max.unwrap_or(1))
            .collect::<Vec<_>>();
        let selected = if preferred.len() >= ctx.min && !preferred.is_empty() {
            preferred
        } else {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        };
        self.trace.push(json!({"choice":"objects","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
        selected
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let selected = SelectFirstDecisionMaker.decide_options(game, ctx);
        self.trace
            .push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":selected}));
        selected
    }
}

fn polymorph_controllers(
    definition: &CardDefinition,
    count: usize,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let mut targets = Vec::new();
    let mut stable = Vec::new();
    for index in [1, 2] {
        let player = PlayerId::from_index(index);
        let victim = game.create_object_from_definition(
            &creature(&format!("Original creature {index}"), 2),
            player,
            Zone::Battlefield,
        );
        targets.push(ironsmith::game_state::Target::Object(victim));
        stable.push(game.object(victim).unwrap().stable_id);
        let remainder =
            CardDefinitionBuilder::new(CardId::new(), &format!("Unrevealed remainder {index}"))
                .card_types(vec![CardType::Artifact])
                .build();
        game.create_object_from_definition(&remainder, player, Zone::Library);
        game.create_object_from_definition(
            &creature(&format!("Replacement creature {index}"), 3),
            player,
            Zone::Library,
        );
        let revealed =
            CardDefinitionBuilder::new(CardId::new(), &format!("Revealed noncreature {index}"))
                .card_types(vec![CardType::Land])
                .build();
        game.create_object_from_definition(&revealed, player, Zone::Library);
    }
    targets.truncate(count);
    let mut dm = ControllerReferenceChoices {
        targets,
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({
        "original_zones":stable.into_iter().map(|stable|game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone))).collect::<Vec<_>>(),
        "replacement_creatures_by_player":([1,2].map(|index|game.battlefield.iter().filter(|id|game.current_controller(**id)==Some(PlayerId::from_index(index))&&game.object(**id).is_some_and(|object|object.name.starts_with("Replacement creature"))).count())),
        "library_lengths":([1,2].map(|index|game.player(PlayerId::from_index(index)).unwrap().library.len())),
        "noncreatures_preserved_in_libraries":([1,2].map(|index|game.player(PlayerId::from_index(index)).unwrap().library.iter().filter(|id|game.object(**id).is_some_and(|object|!object.card_types.contains(&CardType::Creature))).count()))
    });
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn decoy_controller_choices(
    definition: &CardDefinition,
    targets: bool,
    accept_draw: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let mut chosen = Vec::new();
    let mut stable = Vec::new();
    for index in [1, 2] {
        let target = game.create_object_from_definition(
            &creature(&format!("Decoy target {index}"), 2),
            PlayerId::from_index(index),
            Zone::Battlefield,
        );
        chosen.push(ironsmith::game_state::Target::Object(target));
        stable.push(game.object(target).unwrap().stable_id);
    }
    for n in 0..5 {
        game.create_object_from_definition(
            &creature(&format!("Draw buffer {n}"), 1),
            alice(),
            Zone::Library,
        );
    }
    let mut dm = ControllerReferenceChoices {
        targets: if targets { chosen } else { Vec::new() },
        accept: accept_draw,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"target_zones":stable.into_iter().map(|stable|game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone))).collect::<Vec<_>>(),
        "alice_hand":game.player(alice()).unwrap().hand.len(),"alice_library":game.player(alice()).unwrap().library.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn handoff_controller_choice(
    definition: &CardDefinition,
    mana_value: u8,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Handoff artifact")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build();
    let target = game.create_object_from_definition(&target_definition, alice(), Zone::Battlefield);
    for n in 0..5 {
        game.create_object_from_definition(
            &creature(&format!("Handoff draw buffer {n}"), 1),
            alice(),
            Zone::Library,
        );
    }
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Object(target)],
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"alice_hand":game.player(alice()).unwrap().hand.len(),"alice_library":game.player(alice()).unwrap().library.len(),
        "artifact_still_on_battlefield":game.object(target).is_some_and(|object|object.zone==Zone::Battlefield),
        "opponent_controls_artifact":game.current_controller(target).is_some_and(|player|player!=alice())});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn fade_away_controller_choice(
    definition: &CardDefinition,
    creature_count: usize,
    pay: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    for index in 0..3 {
        let player = PlayerId::from_index(index);
        if usize::from(index) < creature_count {
            game.create_object_from_definition(
                &creature(&format!("Taxed creature {index}"), 2),
                player,
                Zone::Battlefield,
            );
        }
        let alternative =
            CardDefinitionBuilder::new(CardId::new(), &format!("Sacrifice alternative {index}"))
                .card_types(vec![CardType::Artifact])
                .build();
        game.create_object_from_definition(&alternative, player, Zone::Battlefield);
        game.player_mut(player)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept: pay,
        trace: Vec::new(),
    };
    let mut before = [0; 3];
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        before = [0, 1, 2].map(|index| {
            game.player(PlayerId::from_index(index))
                .unwrap()
                .mana_pool
                .total()
        });
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"taxed_creatures_on_battlefield":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Taxed creature"))).count(),
        "alternative_artifacts_on_battlefield":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Sacrifice alternative"))).count(),
        "resolution_mana_spent":([0,1,2].map(|index|before[index].saturating_sub(game.player(PlayerId::from_index(index as u8)).unwrap().mana_pool.total())))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_multiplayer_object_controller_contexts() {
    let names = [
        "Chaos Mutation",
        "Divergent Transformations",
        "Decoy Gambit",
        "Fateful Handoff",
        "Fade Away",
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
        if name == "Chaos Mutation" || name == "Divergent Transformations" {
            let counts = if name == "Chaos Mutation" {
                vec![0, 1, 2]
            } else {
                vec![2]
            };
            for count in counts {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"target_count":count,"opposing_players":2,"each_library_top":"noncreature, creature, noncreature"}),
                    json!({"original_zones":([0,1].map(|index|if index<count {"Exile"}else{"Battlefield"})),
                        "replacement_creatures_by_player":([0,1].map(|index|usize::from(index<count))),"library_lengths":([0,1].map(|index|if index<count {2}else{3})),
                        "noncreatures_preserved_in_libraries":[2,2]}),
                    polymorph_controllers(&definition, count, &mut trace),
                    "Actual legal normal instant cast with zero/one/two targets as permitted; opposing creatures have different controllers and owner/controller agree; real exile, reveal and put-onto-battlefield resolution",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Decoy Gambit" {
            for (targets, accept) in [(false, false), (true, false), (true, true)] {
                let mut trace = Vec::new();
                let draws = if targets && accept { 2 } else { 0 };
                record(
                    &mut rows,
                    &name,
                    json!({"choose_one_target_per_opponent":targets,"controllers_accept_draw_instead":accept,"caster_library_size":5}),
                    json!({"target_zones":([0,1].map(|_|if targets&&!accept {"Hand"}else{"Battlefield"})),"alice_hand":draws,"alice_library":5-draws}),
                    decoy_controller_choices(&definition, targets, accept, &mut trace),
                    "Actual legal normal cast; deliberate zero or one target per opponent, explicit unless choice, five-card caster library; targets owned by their controllers",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Fateful Handoff" {
            for mana_value in [0, 3] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"target_artifact_mana_value":mana_value,"target_controller_is_caster":true,"caster_library_size":5}),
                    json!({"alice_hand":mana_value,"alice_library":5-mana_value,"artifact_still_on_battlefield":true,"opponent_controls_artifact":true}),
                    handoff_controller_choice(&definition, mana_value, &mut trace),
                    "Actual legal normal cast targeting caster-controlled artifact; sufficient draw library; no opponent is a target in the oracle text; default first legal non-target opponent choice if offered",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            for (count, pay) in [(0, false), (3, false), (3, true)] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"creatures_on_battlefield":count,"each_controller_has_artifact_alternative":true,"accept_one_mana_payment":pay}),
                    json!({"taxed_creatures_on_battlefield":count,"alternative_artifacts_on_battlefield":if pay {3}else{3-count},
                        "resolution_mana_spent":([0,1,2].map(|index|usize::from(pay&&index<count)))}),
                    fade_away_controller_choice(&definition, count, pay, &mut trace),
                    "Actual legal normal cast; either no creatures or one per player; each controller has one available mana and a noncreature sacrifice alternative; explicit payment choice and preferred artifact sacrifice",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        }
    }
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
    let report = json!({"scope":"Twelve legal cast scenarios for five multiplayer object/controller-context candidates",
        "provenance":{"binary":binary,"binary_sha256":sha,"compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},
        "limitations":"Targets, libraries, spare mana and sacrifice alternatives are seeded. No response invalidates targets. Successful report generation is not a semantics pass.","rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit/multiplayer-controller-context-reproductions.json");
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Multiplayer controller context report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

fn write_context_report(filename: &str, scope: &str, limitations: &str, rows: Vec<Value>) {
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
    let report = json!({"scope":scope,"provenance":{"binary":binary,"binary_sha256":sha,
        "compiled_via":"ironsmith_registry::compile_builder_to_artifact","unique_card_ids":true,"seed":SEED},"limitations":limitations,"rows":rows});
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reports/runtime-audit")
        .join(filename);
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("Runtime context report: {}", out.display());
    for row in report["rows"].as_array().unwrap() {
        println!("{row}");
    }
}

fn optional_creature_etb_target(
    definition: &CardDefinition,
    choose: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Available 4/4 creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let stable = game.object(target).unwrap().stable_id;
    for n in 0..3 {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), &format!("Mill buffer {n}"))
                .card_types(vec![CardType::Land])
                .build(),
            bob,
            Zone::Library,
        );
    }
    let mut dm = ControllerReferenceChoices {
        targets: if choose {
            vec![ironsmith::game_state::Target::Object(target)]
        } else {
            Vec::new()
        },
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let target_current = game.find_object_by_stable_id(stable);
    let actual = json!({"target_zone":target_current.and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "target_battlefield_power":target_current.filter(|id|game.object(*id).is_some_and(|object|object.zone==Zone::Battlefield)).and_then(|id|game.calculated_power(id)),
        "bob_life":game.player(bob).unwrap().life,"bob_library":game.player(bob).unwrap().library.len(),
        "milled_cards":game.player(bob).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Mill buffer"))).count()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn guidelight_search(
    definition: &CardDefinition,
    accept: bool,
    mana_value: u8,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Searchable artifact")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build();
    let target = game.create_object_from_definition(&artifact, alice(), Zone::Library);
    let stable = game.object(target).unwrap().stable_id;
    game.create_object_from_definition(
        &creature("Ineligible creature in library", 1),
        alice(),
        Zone::Library,
    );
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"artifact_zone":game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "library_length":game.player(alice()).unwrap().library.len(),"hand_length":game.player(alice()).unwrap().hand.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn orchid_search(
    definition: &CardDefinition,
    choose: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let nonbasic = CardDefinitionBuilder::new(CardId::new(), "Nonbasic land target")
        .card_types(vec![CardType::Land])
        .build();
    let target = game.create_object_from_definition(&nonbasic, bob, Zone::Battlefield);
    let stable = game.object(target).unwrap().stable_id;
    for n in 0..2 {
        let basic = CardDefinitionBuilder::new(CardId::new(), &format!("Orchid Forest {n}"))
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Basic])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.create_object_from_definition(&basic, bob, Zone::Library);
    }
    let mut dm = ControllerReferenceChoices {
        targets: if choose {
            vec![ironsmith::game_state::Target::Object(target)]
        } else {
            Vec::new()
        },
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let lands = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.name.starts_with("Orchid Forest"))
        })
        .collect::<Vec<_>>();
    let actual = json!({"target_zone":game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "searched_lands":lands.len(),"searched_lands_tapped":lands.iter().filter(|id|game.is_tapped(**id)).count(),
        "searched_lands_controlled_by_bob":lands.iter().filter(|id|game.current_controller(**id)==Some(bob)).count(),"bob_library":game.player(bob).unwrap().library.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn pendant_owner_controller(
    definition: &CardDefinition,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    for index in [0, 1] {
        let player = PlayerId::from_index(index);
        let land = CardDefinitionBuilder::new(CardId::new(), &format!("Hand land {index}"))
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_definition(&land, player, Zone::Hand);
        for n in 0..3 {
            game.create_object_from_definition(
                &creature(&format!("Pendant draw buffer {index}/{n}"), 1),
                player,
                Zone::Library,
            );
        }
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept,
        trace: Vec::new(),
    };
    let mut source = None;
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        let id = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == definition.name())
            })
            .ok_or("Pendant did not enter battlefield")?;
        source = Some(id);
        dm.trace.push(json!({"stage":"after_cast","pendant_controller":format!("{:?}",game.current_controller(id)),"pendant_owner":format!("{:?}",game.object(id).unwrap().owner)}));
        if game.current_controller(id) != Some(bob) {
            return Err("cast fixture did not select Bob as Pendant controller".into());
        }
        game.turn.priority_player = Some(bob);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, bob).expect("fixture has complete replacement state")
            .into_iter()
            .find(|action| matches!(action,LegalAction::ActivateAbility{source,..} if *source==id))
            .ok_or("Pendant activation was not legal")?;
        let mut activation = announce_chosen(&mut game, action, &mut dm)?;
        finish_with_priority(&mut game, &mut activation, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"pendant_controlled_by_bob":source.is_some_and(|id|game.current_controller(id)==Some(bob)),
        "pendant_owned_by_alice":source.and_then(|id|game.object(id)).is_some_and(|object|object.owner==alice()),
        "alice_hand":game.player(alice()).unwrap().hand.len(),"bob_hand":game.player(bob).unwrap().hand.len(),
        "alice_library":game.player(alice()).unwrap().library.len(),"bob_library":game.player(bob).unwrap().library.len(),
        "hand_lands_in_play_by_controller":([0,1].map(|index|game.battlefield.iter().filter(|id|game.current_controller(**id)==Some(PlayerId::from_index(index))&&game.object(**id).is_some_and(|object|object.name.starts_with("Hand land"))).count()))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_optional_etb_and_owner_contexts() {
    let names = [
        "Solitude",
        "Zulaport Duelist",
        "Guidelight Pathmaker",
        "Pendant of Prosperity",
        "White Orchid Phantom",
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
        if name == "Solitude" || name == "Zulaport Duelist" {
            for choose in [false, true] {
                let mut trace = Vec::new();
                let exile = name == "Solitude" && choose;
                let mill = name == "Zulaport Duelist" && choose;
                record(
                    &mut rows,
                    &name,
                    json!({"select_optional_target":choose,"available_target":"opponent 4/4 creature","target_controller_library":3}),
                    json!({"target_zone":if exile {"Exile"}else{"Battlefield"},"target_battlefield_power":if exile {None}else{Some(if mill {2}else{4})},
                        "bob_life":if exile {24}else{20},"bob_library":if mill {1}else{3},"milled_cards":if mill {2}else{0}}),
                    optional_creature_etb_target(&definition, choose, &mut trace),
                    "Actual paid normal cast and real ETB; explicit zero or one legal other-creature target; sufficient opposing library; Solitude evoke is outside scope",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Guidelight Pathmaker" {
            for (accept, mana_value) in [(false, 2), (true, 2), (true, 3)] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"accept_search":accept,"available_artifact_mana_value":mana_value,"library_contains_other_ineligible_card":true}),
                    json!({"artifact_zone":if !accept {"Library"}else if mana_value<=2 {"Battlefield"}else{"Hand"},"library_length":if accept {1}else{2},"hand_length":usize::from(accept&&mana_value>2)}),
                    guidelight_search(&definition, accept, mana_value, &mut trace),
                    "Actual paid normal cast and real ETB; explicit optional search with available legal artifact and positive controls on both sides of the mana-value threshold",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "White Orchid Phantom" {
            for (choose, accept) in [(false, false), (true, false), (true, true)] {
                let mut trace = Vec::new();
                let count = usize::from(choose && accept);
                record(
                    &mut rows,
                    &name,
                    json!({"choose_nonbasic_target":choose,"accept_search":accept,"target_controller_basic_lands_in_library":2}),
                    json!({"target_zone":if choose {"Graveyard"}else{"Battlefield"},"searched_lands":count,"searched_lands_tapped":count,"searched_lands_controlled_by_bob":count,"bob_library":2-count}),
                    orchid_search(&definition, choose, accept, &mut trace),
                    "Actual paid normal cast and real ETB; seeded legal nonbasic land plus two basic lands in its controller's library; explicit zero/one target and optional search choices",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            for accept in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"owner":"Alice","chosen_controller":"Bob","both_players_have_hand_land_and_three_card_library":true,"accept_both_land_placements":accept}),
                    json!({"pendant_controlled_by_bob":true,"pendant_owned_by_alice":true,"alice_hand":if accept {1}else{2},"bob_hand":if accept {1}else{2},"alice_library":2,"bob_library":2,"hand_lands_in_play_by_controller":if accept {[1,1]}else{[0,0]}}),
                    pendant_owner_controller(&definition, accept, &mut trace),
                    "Actual paid cast with opposing ETB control choice, then actual legal two-mana tap activation by Bob; separate owner and controller with populated hands/libraries, explicit optional land choices",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        }
    }
    write_context_report(
        "optional-etb-owner-context-reproductions.json",
        "Twelve legal casting/activation scenarios for five optional ETB and owner-context candidates",
        "Seeded legal targets and hidden-zone resources; no target invalidation. Solitude normal cost only; vehicle crew, evoke and alternate casts outside scope. Successful report generation is not a semantics pass.",
        rows,
    );
}

fn processor_with_resource(
    definition: &CardDefinition,
    available: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let target_definition = CardDefinitionBuilder::new(CardId::new(), "Processor target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
    let target_stable = game.object(target).unwrap().stable_id;
    for index in [1, 2] {
        let card =
            CardDefinitionBuilder::new(CardId::new(), &format!("Opponent hand card {index}"))
                .card_types(vec![CardType::Artifact])
                .build();
        game.create_object_from_definition(&card, PlayerId::from_index(index), Zone::Hand);
    }
    let fuel = available.then(|| {
        game.create_object_from_definition(
            &creature("Opponent-owned processing fuel", 2),
            bob,
            Zone::Exile,
        )
    });
    let fuel_stable = fuel.map(|id| game.object(id).unwrap().stable_id);
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Object(target)],
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let current_target = game.find_object_by_stable_id(target_stable);
    let actual = json!({"processing_fuel_zone":fuel_stable.and_then(|stable|game.find_object_by_stable_id(stable)).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "target_zone":current_target.and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "target_battlefield_power":current_target.filter(|id|game.object(*id).is_some_and(|object|object.zone==Zone::Battlefield)).and_then(|id|game.calculated_power(id)),
        "opponent_hand_lengths":([1,2].map(|index|game.player(PlayerId::from_index(index)).unwrap().hand.len())),
        "alice_life":game.player(alice()).unwrap().life,
        "source_on_battlefield":game.battlefield.iter().any(|id|game.object(*id).is_some_and(|object|object.name==definition.name()))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

struct ResourceBranchChoices(ControllerReferenceChoices);
impl DecisionMaker for ResourceBranchChoices {
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        if ctx.description.to_lowercase().contains("offspring") {
            self.0.trace.push(json!({"choice":"boolean","context":format!("{ctx:?}"),"chosen":false,"reason":"offspring outside this ETB scenario"}));
            false
        } else {
            self.0.decide_boolean(game, ctx)
        }
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.0.decide_targets(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.0.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.min == 0
            && ctx.description.to_lowercase().contains("optional costs")
            && ctx
                .options
                .iter()
                .any(|option| option.description.to_lowercase().contains("offspring"))
        {
            self.0.trace.push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":[],"reason":"offspring outside this ETB scenario"}));
            return Vec::new();
        }
        if ctx.description.to_lowercase().contains("offspring")
            || ctx.description.to_lowercase().contains("additional cost")
        {
            let skipped = ctx
                .options
                .iter()
                .filter(|option| {
                    option.legal
                        && ["don't", "do not", "decline", "skip", "without", "none"]
                            .iter()
                            .any(|word| option.description.to_lowercase().contains(word))
                })
                .map(|option| option.index)
                .take(ctx.max)
                .collect::<Vec<_>>();
            if !skipped.is_empty() {
                self.0.trace.push(json!({"choice":"options","context":format!("{ctx:?}"),"selected":skipped,"reason":"offspring outside this ETB scenario"}));
                return skipped;
            }
        }
        self.0.decide_options(game, ctx)
    }
}

fn bodyguard_forage_resource(
    definition: &CardDefinition,
    resource: &str,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    if resource == "food" {
        let food = CardDefinitionBuilder::new(CardId::new(), "Forage Food")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Food])
            .token()
            .build();
        game.create_object_from_definition(&food, alice(), Zone::Battlefield);
    }
    if resource == "graveyard" {
        for n in 0..3 {
            game.create_object_from_definition(
                &creature(&format!("Forage grave card {n}"), 1),
                alice(),
                Zone::Graveyard,
            );
        }
    }
    let mut dm = ResourceBranchChoices(ControllerReferenceChoices {
        targets: Vec::new(),
        accept,
        trace: Vec::new(),
    });
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.0.trace);
    let sources = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == definition.name())
        })
        .collect::<Vec<_>>();
    let actual = json!({"source_count":sources.len(),"source_counters":sources.iter().map(|id|game.counter_count(*id,CounterType::PlusOnePlusOne)).collect::<Vec<_>>(),
        "foods_on_battlefield":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.subtypes.contains(&Subtype::Food))).count(),
        "forage_cards_in_graveyard":game.player(alice()).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Forage grave card"))).count(),
        "forage_cards_in_exile":game.exile.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Forage grave card"))).count()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn elven_passage_resource(
    definition: &CardDefinition,
    elf: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let source = game.create_object_from_definition(definition, alice(), Zone::Battlefield);
    let stable = game.object(source).unwrap().stable_id;
    let forest = CardDefinitionBuilder::new(CardId::new(), "Passage Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![ironsmith::Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    game.create_object_from_definition(&forest, alice(), Zone::Library);
    if elf {
        let elf = CardDefinitionBuilder::new(CardId::new(), "Beholdable Elf")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_definition(&elf, alice(), Zone::Hand);
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let action = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state")
            .into_iter()
            .find(
                |action| matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source),
            )
            .ok_or("Elven Passage activation not legal")?;
        let mut queue = announce_chosen(&mut game, action, &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let lands = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Passage Forest")
        })
        .collect::<Vec<_>>();
    let actual = json!({"source_zone":game.find_object_by_stable_id(stable).and_then(|id|game.object(id)).map(|object|format!("{:?}",object.zone)),
        "alice_life":game.player(alice()).unwrap().life,"searched_lands":lands.len(),"searched_lands_tapped":lands.iter().filter(|id|game.is_tapped(**id)).count(),
        "elf_still_in_hand":game.player(alice()).unwrap().hand.iter().any(|id|game.object(*id).is_some_and(|object|object.name=="Beholdable Elf"))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_optional_resource_contexts() {
    let names = [
        "Mind Raker",
        "Murk Strider",
        "Ruin Processor",
        "Wasteland Strangler",
        "Bushy Bodyguard",
        "Elven Passage",
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
        if name == "Bushy Bodyguard" {
            for (resource, accept) in [
                ("none", false),
                ("food", false),
                ("food", true),
                ("graveyard", true),
            ] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"forage_resource":resource,"accept_forage":accept,"offspring_intended_declined":true}),
                    json!({"source_count":1,"source_counters":vec![if accept {2}else{0}],"foods_on_battlefield":usize::from(resource=="food"&&!accept),
                        "forage_cards_in_graveyard":if resource=="graveyard"&&!accept {3}else{0},"forage_cards_in_exile":if resource=="graveyard"&&accept {3}else{0}}),
                    bodyguard_forage_resource(&definition, resource, accept, &mut trace),
                    "Actual paid normal creature cast and live ETB; offspring explicitly declined if offered; optional forage skipped or supplied with Food token/three graveyard cards",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Elven Passage" {
            for (elf, accept) in [(false, false), (true, false), (true, true)] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"elf_available_in_hand":elf,"accept_behold":accept,"basic_land_available_in_library":true}),
                    json!({"source_zone":"Graveyard","alice_life":19,"searched_lands":1,"searched_lands_tapped":usize::from(!accept),"elf_still_in_hand":elf}),
                    elven_passage_resource(&definition, elf, accept, &mut trace),
                    "Source land seeded on battlefield; actual legal activated ability with tap, life and sacrifice costs paid, real basic-land search and optional Elf reveal",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            for (available, accept) in [(false, false), (true, false), (true, true)] {
                let mut trace = Vec::new();
                let processed = available && accept;
                let bounce = processed && name == "Murk Strider";
                let discard = processed && name == "Mind Raker";
                record(
                    &mut rows,
                    &name,
                    json!({"opponent_owned_card_in_exile":available,"accept_processing":accept,"each_opponent_hand_size":1,"target_is_opposing_5_5":true}),
                    json!({"processing_fuel_zone":if available {Some(if processed {"Graveyard"}else{"Exile"})}else{None},
                        "target_zone":if bounce {"Hand"}else{"Battlefield"},"target_battlefield_power":if bounce {None}else{Some(if processed&&name=="Wasteland Strangler" {2}else{5})},
                        "opponent_hand_lengths":[if discard {0}else if bounce {2}else{1},if discard {0}else{1}],
                        "alice_life":if processed&&name=="Ruin Processor" {25}else{20},"source_on_battlefield":true}),
                    processor_with_resource(&definition, available, accept, &mut trace),
                    "Actual paid normal creature cast; live cast trigger for Ruin Processor or ETB for other Processors; valid opposing exile ownership, legal creature target and populated opposing hands; explicit decline/accept",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        }
    }
    write_context_report(
        "optional-resource-context-reproductions.json",
        "Nineteen resource-complete or legitimate-decline scenarios across four Processors, forage and behold",
        "Resources and Elven Passage source are seeded; all actions and costs are actual legal actions. No acceptance is forced when its required resource is unavailable. Successful report generation is not a semantics pass.",
        rows,
    );
}

fn incriminate_same_controller(
    definition: &CardDefinition,
    controller: PlayerId,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let mut targets = Vec::new();
    let mut stable = Vec::new();
    for n in 0..2 {
        let id = game.create_object_from_definition(
            &creature(&format!("Incriminate target {n}"), 2),
            controller,
            Zone::Battlefield,
        );
        targets.push(ironsmith::game_state::Target::Object(id));
        stable.push(game.object(id).unwrap().stable_id);
    }
    let outside = game.create_object_from_definition(
        &creature("Untargeted creature", 2),
        controller,
        Zone::Battlefield,
    );
    let mut dm = ControllerReferenceChoices {
        targets,
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let zones = stable
        .into_iter()
        .filter_map(|stable| game.find_object_by_stable_id(stable))
        .filter_map(|id| game.object(id).map(|object| object.zone))
        .collect::<Vec<_>>();
    let actual = json!({"targeted_creatures_on_battlefield":zones.iter().filter(|zone|**zone==Zone::Battlefield).count(),"targeted_creatures_in_graveyard":zones.iter().filter(|zone|**zone==Zone::Graveyard).count(),"untargeted_creature_untouched":game.object(outside).is_some_and(|object|object.zone==Zone::Battlefield)});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn invigorate_cost_path(
    definition: &CardDefinition,
    alternative: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    use ironsmith::alternative_cast::CastingMethod;
    let mut game = setup();
    let target = game.create_object_from_definition(
        &creature("Invigorate target", 2),
        alice(),
        Zone::Battlefield,
    );
    let forest = CardDefinitionBuilder::new(CardId::new(), "Controlled Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![ironsmith::Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    game.create_object_from_definition(&forest, alice(), Zone::Battlefield);
    let source = game.create_object_from_definition(definition, alice(), Zone::Hand);
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(alice()).unwrap().mana_pool.add(symbol, 12);
    }
    let legal = compute_legal_actions(&game, alice()).expect("fixture has complete replacement state");
    trace.push(json!({"stage":"legal_cast_actions","compiled_alternative_costs":format!("{:?}",definition.alternative_casts),"compiled_spell_effect":format!("{:?}",definition.spell_effect),"actions":legal.iter().filter(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).map(|action|format!("{action:?}")).collect::<Vec<_>>()}));
    let action=legal.into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source&&if alternative {matches!(casting_method,CastingMethod::Alternative(_))}else{matches!(casting_method,CastingMethod::Normal)}));
    let available = action.is_some();
    let before = game.player(alice()).unwrap().mana_pool.total();
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Object(target)],
        accept: alternative,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        if let Some(action) = action {
            let mut queue = announce_chosen(&mut game, action, &mut dm)?;
            finish_with_priority(&mut game, &mut queue, &mut dm)?;
        }
        Ok(())
    })();
    trace.extend(dm.trace);
    let mut opponent_life =
        [1, 2].map(|index| game.player(PlayerId::from_index(index)).unwrap().life);
    opponent_life.sort();
    let actual = json!({"requested_cast_action_available":available,"mana_spent":before-game.player(alice()).unwrap().mana_pool.total(),"target_power":game.calculated_power(target),"target_toughness":game.calculated_toughness(target),"alice_life":game.player(alice()).unwrap().life,"opponent_life_sorted":opponent_life});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

struct RevealCountChoices {
    inner: ControllerReferenceChoices,
    count: u32,
}
impl DecisionMaker for RevealCountChoices {
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let chosen = self.count.clamp(ctx.min, ctx.max);
        self.inner
            .trace
            .push(json!({"choice":"number","context":format!("{ctx:?}"),"chosen":chosen}));
        chosen
    }
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(game, ctx)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.inner.decide_targets(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.inner.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(game, ctx)
    }
}

fn nightshade_reveal_count(
    definition: &CardDefinition,
    black_cards: usize,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let witness = CardDefinitionBuilder::new(CardId::new(), "Nightshade 5/5 target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let target =
        game.create_object_from_definition(&witness, PlayerId::from_index(1), Zone::Battlefield);
    for n in 0..black_cards {
        let black =
            CardDefinitionBuilder::new(CardId::new(), &format!("Revealable black card {n}"))
                .card_types(vec![CardType::Creature])
                .color_indicator(ironsmith::color::ColorSet::BLACK)
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]))
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
        game.create_object_from_definition(&black, alice(), Zone::Hand);
    }
    let mut dm = RevealCountChoices {
        inner: ControllerReferenceChoices {
            targets: vec![ironsmith::game_state::Target::Object(target)],
            accept,
            trace: Vec::new(),
        },
        count: black_cards as u32,
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.inner.trace);
    let actual = json!({"black_cards_still_in_hand":game.player(alice()).unwrap().hand.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Revealable black card"))).count(),"target_power":game.calculated_power(target),"target_toughness":game.calculated_toughness(target)});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn redeem_target_count(
    definition: &CardDefinition,
    count: usize,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let mut witnesses = Vec::new();
    for n in 0..3 {
        let card = CardDefinitionBuilder::new(CardId::new(), &format!("Damage witness {n}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        witnesses.push(game.create_object_from_definition(&card, alice(), Zone::Battlefield));
    }
    let mut dm = ControllerReferenceChoices {
        targets: witnesses
            .iter()
            .take(count)
            .copied()
            .map(ironsmith::game_state::Target::Object)
            .collect(),
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        let damage = CardDefinitionBuilder::new(CardId::new(), "Actual damage follow-up")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .with_spell_effect(
                witnesses
                    .iter()
                    .map(|id| {
                        ironsmith::Effect::deal_damage(
                            3,
                            ironsmith::target::ChooseSpec::SpecificObject(*id),
                        )
                    })
                    .collect(),
            )
            .build();
        let mut queue = cast_from_hand(&mut game, &damage, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"damage_marked":witnesses.iter().map(|id|game.damage_on(*id)).collect::<Vec<_>>(),"witnesses_survived":witnesses.iter().all(|id|game.object(*id).is_some_and(|object|object.zone==Zone::Battlefield))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn unexpected_request_equipment(
    definition: &CardDefinition,
    available: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let target = game.create_object_from_definition(
        &creature("Borrowed target", 2),
        PlayerId::from_index(1),
        Zone::Battlefield,
    );
    game.tap(target);
    let equipment = available.then(|| {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::new(), "Available Equipment")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![Subtype::Equipment])
                .build(),
            alice(),
            Zone::Battlefield,
        )
    });
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Object(target)],
        accept,
        trace: Vec::new(),
    };
    let mut attached_before_end = false;
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        attached_before_end = equipment
            .and_then(|id| game.object(id))
            .is_some_and(|object| {
                object.attached_to == Some(ironsmith::object::AttachmentTarget::Object(target))
            });
        dm.trace.push(json!({"stage":"after_spell","equipment_attached":attached_before_end,"equipment_id":format!("{equipment:?}"),"target_id":format!("{target:?}"),"target_controller":format!("{:?}",game.current_controller(target)),"delayed_triggers":format!("{:?}",game.effect_store.delayed_triggers)}));
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        generate_and_queue_step_triggers(&mut game, &mut queue);
        let resolved = finish_with_priority(&mut game, &mut queue, &mut dm)?;
        dm.trace.push(json!({"stage":"after_end_step","resolved_stack_entries":resolved,"delayed_triggers_remaining":game.effect_store.delayed_triggers.len()}));
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"target_controlled_by_caster_at_end_step":game.current_controller(target)==Some(alice()),"target_untapped":!game.is_tapped(target),"target_has_haste":game.object_has_static_ability_id(target,ironsmith::static_abilities::StaticAbilityId::Haste),"equipment_attached_before_end":attached_before_end,"equipment_unattached_after_end_trigger":equipment.and_then(|id|game.object(id)).is_none_or(|object|object.attached_to.is_none())});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_remaining_target_cost_and_optional_contexts() {
    let names = [
        "Incriminate",
        "Invigorate",
        "Nightshade Assassin",
        "Redeem",
        "Unexpected Request",
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
        if name == "Incriminate" {
            for index in [0, 1] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"target_controller_index":index,"two_targets_share_controller":true,"untargeted_creature_same_controller":true}),
                    json!({"targeted_creatures_on_battlefield":1,"targeted_creatures_in_graveyard":1,"untargeted_creature_untouched":true}),
                    incriminate_same_controller(
                        &definition,
                        PlayerId::from_index(index),
                        &mut trace,
                    ),
                    "Actual legal normal cast targeting exactly two creatures of the same player; third untargeted creature provides a sacrifice-exclusion control; chooser trace retained",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Invigorate" {
            for alternative in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"request_alternative_life_gain_cost":alternative,"caster_controls_forest":true,"legal_creature_target_present":true}),
                    json!({"requested_cast_action_available":true,"mana_spent":if alternative {0}else{3},"target_power":6,"target_toughness":6,"alice_life":20,"opponent_life_sorted":if alternative {[20,23]}else{[20,20]}}),
                    invigorate_cost_path(&definition, alternative, &mut trace),
                    "Computed legal actions with controlled Forest and valid target; explicit normal/alternate casting method selection, mana accounting, opponent life and actual pump outcome; missing alternate action recorded as an expected-behavior mismatch",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Nightshade Assassin" {
            for (black, accept) in [(2, false), (0, true), (2, true)] {
                let mut trace = Vec::new();
                let reduction = if accept { black } else { 0 };
                record(
                    &mut rows,
                    &name,
                    json!({"black_cards_in_hand":black,"accept_reveal":accept,"target_is_opposing_5_5":true}),
                    json!({"black_cards_still_in_hand":black,"target_power":5-reduction,"target_toughness":5-reduction}),
                    nightshade_reveal_count(&definition, black, accept, &mut trace),
                    "Actual paid normal cast, live ETB and legal opposing target; optional reveal declined or supplied with zero/two black hand cards, selecting available black cards if offered; madness outside scope",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else if name == "Redeem" {
            for count in [0, 1, 2] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"chosen_creatures":count,"witness_creatures":3,"actual_followup_damage_each":3}),
                    json!({"damage_marked":(0..3).map(|index|if index<count {0}else{3}).collect::<Vec<_>>(),"witnesses_survived":true}),
                    redeem_target_count(&definition, count, &mut trace),
                    "Actual paid normal cast with zero/one/two legal targets, then a legally cast generic damage spell dealing three to each of three witnesses in the same turn",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            for (available, accept) in [(false, false), (true, false), (true, true)] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"equipment_available":available,"accept_attachment":accept,"borrowed_target_initially_tapped":true}),
                    json!({"target_controlled_by_caster_at_end_step":true,"target_untapped":true,"target_has_haste":true,"equipment_attached_before_end":available&&accept,"equipment_unattached_after_end_trigger":true}),
                    unexpected_request_equipment(&definition, available, accept, &mut trace),
                    "Actual paid normal cast with opposing tapped creature and optional controlled Equipment; explicit attachment choice then real next-end-step event and delayed trigger resolution; cleanup control reversion outside scope",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        }
    }
    write_context_report(
        "target-cost-optional-context-reproductions.json",
        "Thirteen targeted legal-action/expected-outcome scenarios across five remaining target, cost and optional-context candidates",
        "Generic setup permanents and damage spell are fixtures. Optional actions have the resources described in each row. Exact chosen opponent for Invigorate is unconstrained; sorted totals require one opponent gaining three. Successful report generation is not a semantics pass.",
        rows,
    );
}

struct PathVoteChoices {
    inner: ControllerReferenceChoices,
    chaos: bool,
    voters: Vec<String>,
}
impl DecisionMaker for PathVoteChoices {
    fn decide_number(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        let chosen = 2_u32.clamp(ctx.min, ctx.max);
        self.inner
            .trace
            .push(json!({"choice":"number","context":format!("{ctx:?}"),"chosen":chosen}));
        chosen
    }
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(game, ctx)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.inner.decide_targets(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.inner.decide_objects(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let vote = if self.chaos { "chaos" } else { "planeswalk" };
        if let Some(option) = ctx
            .options
            .iter()
            .find(|option| option.legal && option.description.eq_ignore_ascii_case(vote))
        {
            self.voters.push(format!("{:?}", ctx.player));
            self.inner
                .trace
                .push(json!({"choice":"vote","player":format!("{:?}",ctx.player),"option":vote}));
            return vec![option.index];
        }
        self.inner.decide_options(game, ctx)
    }
}

fn path_vote_scenario(
    definition: &CardDefinition,
    planechase: bool,
    chaos: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    if planechase {
        let planes = (0..30)
            .map(|n| {
                (
                    CardDefinitionBuilder::new(
                        CardId::new(),
                        &format!("Neutral planar fixture {n}"),
                    )
                    .card_types(vec![CardType::Plane])
                    .build(),
                    ironsmith::game_state::PlanarCardKind::Plane,
                )
            })
            .collect();
        game.enable_planechase_communal(planes)?;
        game.reveal_starting_plane()?;
    }
    let initial_planes = game.face_up_planar_objects().to_vec();
    let name = definition.name();
    if name == "Path of the Animist" {
        for n in 0..3 {
            let basic = CardDefinitionBuilder::new(CardId::new(), &format!("Path basic {n}"))
                .card_types(vec![CardType::Land])
                .supertypes(vec![ironsmith::Supertype::Basic])
                .subtypes(vec![Subtype::Forest])
                .build();
            game.create_object_from_definition(&basic, alice(), Zone::Library);
        }
    } else if name == "Path of the Enigma" {
        for n in 0..6 {
            game.create_object_from_definition(
                &creature(&format!("Enigma draw card {n}"), 1),
                PlayerId::from_index(1),
                Zone::Library,
            );
        }
    } else if name == "Path of the Pyromancer" {
        for n in 0..2 {
            game.create_object_from_definition(
                &creature(&format!("Pyromancer discard card {n}"), 1),
                alice(),
                Zone::Hand,
            );
        }
        for n in 0..4 {
            game.create_object_from_definition(
                &creature(&format!("Pyromancer draw card {n}"), 1),
                alice(),
                Zone::Library,
            );
        }
    } else if name == "Path of the Schemer" {
        for index in [0, 1, 2] {
            let player = PlayerId::from_index(index);
            let remainder =
                CardDefinitionBuilder::new(CardId::new(), &format!("Unmilled remainder {index}"))
                    .card_types(vec![CardType::Land])
                    .build();
            game.create_object_from_definition(&remainder, player, Zone::Library);
            let artifact =
                CardDefinitionBuilder::new(CardId::new(), &format!("Path mill artifact {index}"))
                    .card_types(vec![CardType::Artifact])
                    .build();
            game.create_object_from_definition(&artifact, player, Zone::Library);
            game.create_object_from_definition(
                &creature(&format!("Path mill creature {index}"), 2),
                player,
                Zone::Library,
            );
        }
    }
    let mut dm = PathVoteChoices {
        inner: ControllerReferenceChoices {
            targets: vec![ironsmith::game_state::Target::Player(PlayerId::from_index(
                1,
            ))],
            accept: true,
            trace: Vec::new(),
        },
        chaos,
        voters: Vec::new(),
    };
    let mut red_before = 0;
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        red_before = game.player(alice()).unwrap().mana_pool.red;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.inner.trace);
    let printed_effect = if name == "Path of the Animist" {
        let lands = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name.starts_with("Path basic"))
            })
            .collect::<Vec<_>>();
        json!({"basic_lands_in_play":lands.len(),"basic_lands_tapped":lands.iter().filter(|id|game.is_tapped(**id)).count(),"library_remaining":game.player(alice()).unwrap().library.len()})
    } else if name == "Path of the Enigma" {
        json!({"target_hand":game.player(PlayerId::from_index(1)).unwrap().hand.len(),"target_library":game.player(PlayerId::from_index(1)).unwrap().library.len()})
    } else if name == "Path of the Ghosthunter" {
        let spirits = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.subtypes.contains(&Subtype::Spirit))
            })
            .collect::<Vec<_>>();
        json!({"spirit_tokens":spirits.len(),"all_are_1_1_flying":spirits.iter().all(|id|game.calculated_power(*id)==Some(1)&&game.calculated_toughness(*id)==Some(1)&&game.object_has_static_ability_id(*id,ironsmith::static_abilities::StaticAbilityId::Flying))})
    } else if name == "Path of the Pyromancer" {
        json!({"discarded_cards":game.player(alice()).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Pyromancer discard"))).count(),"hand_after_draw":game.player(alice()).unwrap().hand.len(),"red_mana_added":game.player(alice()).unwrap().mana_pool.red.saturating_sub(red_before)})
    } else {
        let returned = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name.starts_with("Path mill creature"))
            })
            .collect::<Vec<_>>();
        json!({"library_lengths":([0,1,2].map(|index|game.player(PlayerId::from_index(index)).unwrap().library.len())),"milled_cards_left_in_graveyards":game.players.iter().flat_map(|player|player.graveyard.iter()).filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Path mill"))).count(),"returned_creatures":returned.len(),"returned_are_artifacts_controlled_by_caster":returned.iter().all(|id|game.current_controller(*id)==Some(alice())&&game.current_card_types(*id).is_some_and(|types|types.contains(&CardType::Artifact)&&types.contains(&CardType::Creature)))})
    };
    let actual = json!({"printed_effect":printed_effect,"voters":dm.voters,"planeswalk_count":game.planeswalk_count(),"face_up_planes":game.face_up_planar_objects().len(),"face_up_plane_changed":game.face_up_planar_objects()!=initial_planes.as_slice()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn omen_of_fire_board(
    definition: &CardDefinition,
    white: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    for index in [0, 1, 2] {
        let player = PlayerId::from_index(index);
        let island = CardDefinitionBuilder::new(CardId::new(), &format!("Omen Island {index}"))
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Basic])
            .subtypes(vec![Subtype::Island])
            .build();
        game.create_object_from_definition(&island, player, Zone::Battlefield);
        let plains = CardDefinitionBuilder::new(
            CardId::new(),
            &format!("Sacrifice alternative Plains {index}"),
        )
        .card_types(vec![CardType::Land])
        .supertypes(vec![ironsmith::Supertype::Basic])
        .subtypes(vec![Subtype::Plains])
        .build();
        game.create_object_from_definition(&plains, player, Zone::Battlefield);
        if white {
            let creature =
                CardDefinitionBuilder::new(CardId::new(), &format!("Omen white creature {index}"))
                    .card_types(vec![CardType::Creature])
                    .color_indicator(ironsmith::color::ColorSet::WHITE)
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
            game.create_object_from_definition(&creature, player, Zone::Battlefield);
        }
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"islands_in_hands":game.players.iter().flat_map(|player|player.hand.iter()).filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Omen Island"))).count(),"plains_in_graveyards":game.players.iter().flat_map(|player|player.graveyard.iter()).filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Sacrifice alternative Plains"))).count(),"white_creatures_survived":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.name.starts_with("Omen white creature"))).count()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_path_vote_and_omen_contexts() {
    let names = [
        "Path of the Animist",
        "Path of the Enigma",
        "Path of the Ghosthunter",
        "Path of the Pyromancer",
        "Path of the Schemer",
        "Omen of Fire",
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
        if name == "Omen of Fire" {
            for white in [false, true] {
                let mut trace = Vec::new();
                record(
                    &mut rows,
                    &name,
                    json!({"each_player_islands":1,"each_player_plains":1,"each_player_white_creatures":usize::from(white),"choose_plains_sacrifices":true}),
                    json!({"islands_in_hands":3,"plains_in_graveyards":if white {3}else{0},"white_creatures_survived":if white {3}else{0}}),
                    omen_of_fire_board(&definition, white, &mut trace),
                    "Actual paid normal instant cast; every player has an Island and Plains, optionally a white creature; explicit preference to sacrifice Plains instead of the white creature",
                    checksum,
                );
                rows.last_mut().unwrap()["execution_trace"] = json!(trace);
            }
        } else {
            let printed = match name.as_str() {
                "Path of the Animist" => {
                    json!({"basic_lands_in_play":2,"basic_lands_tapped":2,"library_remaining":1})
                }
                "Path of the Enigma" => json!({"target_hand":4,"target_library":2}),
                "Path of the Ghosthunter" => json!({"spirit_tokens":2,"all_are_1_1_flying":true}),
                "Path of the Pyromancer" => {
                    json!({"discarded_cards":2,"hand_after_draw":3,"red_mana_added":2})
                }
                _ => {
                    json!({"library_lengths":[1,1,1],"milled_cards_left_in_graveyards":5,"returned_creatures":1,"returned_are_artifacts_controlled_by_caster":true})
                }
            };
            for planechase in [false, true] {
                for chaos in [false, true] {
                    let mut trace = Vec::new();
                    record(
                        &mut rows,
                        &name,
                        json!({"planechase_enabled":planechase,"all_three_votes":if chaos {"chaos"}else{"planeswalk"},"caster_is_active_player":true,"ghosthunter_x":2}),
                        json!({"printed_effect":printed,"voters":["PlayerId(0)","PlayerId(1)","PlayerId(2)"],"planeswalk_count":if planechase {Some(u64::from(!chaos))}else{None},"face_up_planes":usize::from(planechase),"face_up_plane_changed":planechase&&!chaos}),
                        path_vote_scenario(&definition, planechase, chaos, &mut trace),
                        "Actual paid normal cast with complete printed-effect resources, explicit X2 where required, and unanimous votes in turn order; ordinary game or configured 30-card communal neutral-plane fixture with revealed starting plane",
                        checksum,
                    );
                    rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                }
            }
        }
    }
    write_context_report(
        "path-vote-omen-context-reproductions.json",
        "Twenty-two actual-cast scenarios across five Path voting spells and Omen of Fire",
        "Planechase fixtures use thirty distinct generic planes without gameplay abilities, so chaos checks continuation/unchanged plane rather than a printed chaos ability. Ordinary-game planeswalking must do nothing per official Doctor Who release notes. No vote ties or out-of-turn Path casts are exercised. Report generation is not a semantics pass.",
        rows,
    );
}

struct DamageSourceChoices {
    inner: ControllerReferenceChoices,
    source: ObjectId,
}
impl DecisionMaker for DamageSourceChoices {
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(game, ctx)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.inner.decide_targets(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .any(|candidate| candidate.legal && candidate.id == self.source)
            && ctx.min <= 1
            && ctx.max.is_none_or(|max| max >= 1)
        {
            self.inner.trace.push(json!({"choice":"damage_source_object","context":format!("{ctx:?}"),"selected":format!("{:?}",self.source)}));
            vec![self.source]
        } else {
            self.inner.decide_objects(game, ctx)
        }
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().find(|option| {
            option.legal
                && (option.object_id == Some(self.source)
                    || option
                        .related_object_ids
                        .as_ref()
                        .is_some_and(|ids| ids.contains(&self.source)))
        }) {
            self.inner.trace.push(json!({"choice":"damage_source_option","context":format!("{ctx:?}"),"selected":option.index,"source":format!("{:?}",self.source)}));
            vec![option.index]
        } else {
            self.inner.decide_options(game, ctx)
        }
    }
}

fn new_way_forward_damage(
    definition: &CardDefinition,
    damage: i32,
    match_source: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    let decoy = game.create_object_from_definition(
        &creature("Different damage source", 2),
        bob,
        Zone::Battlefield,
    );
    for n in 0..5 {
        game.create_object_from_definition(
            &creature(&format!("Prevention draw buffer {n}"), 1),
            alice(),
            Zone::Library,
        );
    }
    let spell = CardDefinitionBuilder::new(CardId::new(), "Incoming damage spell")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .with_spell_effect(vec![ironsmith::Effect::deal_damage(
            damage,
            ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(
                alice(),
            )),
        )])
        .build();
    let mut first = ControllerReferenceChoices {
        targets: Vec::new(),
        accept: true,
        trace: Vec::new(),
    };
    let mut queue = cast_from_hand(&mut game, &spell, bob, &mut first)?;
    let incoming = game
        .stack
        .last()
        .ok_or("incoming spell absent from stack")?
        .object_id;
    trace.extend(first.trace);
    trace.push(json!({"stage":"incoming_spell_on_stack","damage":damage,"incoming_source":format!("{incoming:?}"),"selected_matching_source":match_source}));
    let mut dm = DamageSourceChoices {
        inner: ControllerReferenceChoices {
            targets: Vec::new(),
            accept: true,
            trace: Vec::new(),
        },
        source: if match_source { incoming } else { decoy },
    };
    let result: Result<(), String> = (|| {
        // Alice has priority to respond to Bob's still-unresolved damage spell.
        let mut response_queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        if game.stack.len() != 2 {
            return Err(format!(
                "expected response and damage spell on stack, got {}",
                game.stack.len()
            ));
        }
        finish_with_priority(&mut game, &mut response_queue, &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.inner.trace);
    let actual = json!({"alice_life":game.player(alice()).unwrap().life,"bob_life":game.player(bob).unwrap().life,"alice_hand":game.player(alice()).unwrap().hand.len(),"alice_library":game.player(alice()).unwrap().library.len(),"stack_empty":game.stack.is_empty()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_new_way_forward_actual_damage() {
    let name = "New Way Forward";
    let names = vec![name.to_owned()];
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        &names,
    )
    .unwrap();
    let payload = &payloads[name][0];
    let builder = ironsmith_compiler::CardDefinitionBuilder::new(
        CardId::new(),
        payload.parse_name.as_deref().unwrap_or(&payload.name),
    );
    let (artifact, definition) =
        ironsmith_registry::compile_builder_to_artifact(builder, &payload.parse_input, false)
            .unwrap();
    let mut rows = Vec::new();
    for (damage, match_source) in [(0, true), (3, true), (3, false)] {
        let prevented = if match_source { damage } else { 0 };
        let mut trace = Vec::new();
        record(
            &mut rows,
            name,
            json!({"incoming_damage":damage,"selected_source_matches_incoming_spell":match_source,"actual_response_cast":true,"caster_library_size":5}),
            json!({"alice_life":20-(damage-prevented),"bob_life":20-prevented,"alice_hand":prevented,"alice_library":5-prevented,"stack_empty":true}),
            new_way_forward_damage(&definition, damage, match_source, &mut trace),
            "Bob legally casts a generic damage sorcery; Alice legally casts New Way Forward in response and explicitly selects the damage source or a different visible source; actual incoming damage, prevention, reflexive triggers and draw processing",
            &artifact.payload_checksum,
        );
        rows.last_mut().unwrap()["execution_trace"] = json!(trace);
    }
    write_context_report(
        "new-way-forward-damage-reproductions.json",
        "Three actual incoming-damage and source-selection scenarios for New Way Forward",
        "Generic damage source spell and unrelated creature are fixtures; prevention source choice is explicit and traceable. No combat, damage replacement or repeated damage in one turn. Successful report generation is not a semantics pass.",
        rows,
    );
}

fn settle_actual_attackers(
    definition: &CardDefinition,
    count: usize,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let bob = PlayerId::from_index(1);
    let mut attackers = Vec::new();
    for n in 0..count {
        attackers.push(game.create_object_from_definition(
            &creature(&format!("Settle attacker {n}"), 2),
            alice(),
            Zone::Battlefield,
        ));
    }
    let idle =
        game.create_object_from_definition(&creature("Settle idle", 2), alice(), Zone::Battlefield);
    let defender =
        game.create_object_from_definition(&creature("Settle defender", 2), bob, Zone::Battlefield);
    for n in 0..3 {
        let land = CardDefinitionBuilder::new(CardId::new(), &format!("Settle basic {n}"))
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Basic])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.create_object_from_definition(&land, alice(), Zone::Library);
    }
    let mut attack_queue = attack(&mut game, &attackers)?;
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Player(alice())],
        accept,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        finish_with_priority(&mut game, &mut attack_queue, &mut dm)?;
        dm.trace.push(
            json!({"stage":"actual_attackers_declared","combat":format!("{:?}",game.combat)}),
        );
        let mut queue = cast_from_hand(&mut game, definition, bob, &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let basics = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|o| o.name.starts_with("Settle basic"))
        })
        .copied()
        .collect::<Vec<_>>();
    let actual = json!({"exiled_attackers":game.exile.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Settle attacker"))).count(),
        "idle_survives":game.object(idle).is_some_and(|o|o.zone==Zone::Battlefield),"defender_survives":game.object(defender).is_some_and(|o|o.zone==Zone::Battlefield),
        "basic_lands_in_play":basics.len(),"basic_lands_tapped":basics.iter().filter(|id|game.is_tapped(**id)).count(),"target_library_remaining":game.player(alice()).unwrap().library.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn stench_actual_plains(
    definition: &CardDefinition,
    counts: [usize; 3],
    pay: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    for index in 0u8..3 {
        let player = PlayerId::from_index(index);
        game.player_mut(player)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 12);
        for n in 0..counts[index as usize] {
            let plains =
                CardDefinitionBuilder::new(CardId::new(), &format!("Stench Plains {index} {n}"))
                    .card_types(vec![CardType::Land])
                    .supertypes(vec![ironsmith::Supertype::Basic])
                    .subtypes(vec![Subtype::Plains])
                    .build();
            game.create_object_from_definition(&plains, player, Zone::Battlefield);
        }
        let land = CardDefinitionBuilder::new(CardId::new(), &format!("Stench Forest {index}"))
            .card_types(vec![CardType::Land])
            .supertypes(vec![ironsmith::Supertype::Basic])
            .subtypes(vec![Subtype::Forest])
            .build();
        game.create_object_from_definition(&land, player, Zone::Battlefield);
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept: pay,
        trace: Vec::new(),
    };
    let mut before = [0u32; 3];
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        before = [0, 1, 2].map(|i| {
            game.player(PlayerId::from_index(i))
                .unwrap()
                .mana_pool
                .total()
        });
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"life_totals":([0,1,2].map(|i|game.player(PlayerId::from_index(i)).unwrap().life)),
        "resolution_mana_paid":([0,1,2].map(|i|before[i as usize].saturating_sub(game.player(PlayerId::from_index(i)).unwrap().mana_pool.total()))),
        "plains_in_graveyards":([0,1,2].map(|i|game.player(PlayerId::from_index(i)).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Stench Plains"))).count())),
        "forests_survive":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Stench Forest"))).count()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

struct SplitPileChoices {
    inner: ControllerReferenceChoices,
    pile_size: usize,
}
impl DecisionMaker for SplitPileChoices {
    fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
        self.inner.decide_boolean(game, ctx)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::game_state::Target> {
        self.inner.decide_targets(game, ctx)
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .all(|o| o.name.starts_with("Split permanent"))
        {
            let selected = ctx
                .candidates
                .iter()
                .filter(|o| o.legal)
                .take(self.pile_size)
                .map(|o| o.id)
                .collect::<Vec<_>>();
            self.inner.trace.push(json!({"choice":"first_pile","context":format!("{ctx:?}"),"selected":format!("{selected:?}")}));
            selected
        } else {
            self.inner.decide_objects(game, ctx)
        }
    }
}
fn split_spoils_piles(
    definition: &CardDefinition,
    count: usize,
    choose_first: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    let mut targets = Vec::new();
    for n in 0..count {
        let id = game.create_object_from_definition(
            &creature(&format!("Split permanent {n}"), 2),
            alice(),
            Zone::Graveyard,
        );
        targets.push(ironsmith::game_state::Target::Object(id));
    }
    let untouched = game.create_object_from_definition(
        &creature("Split untargeted permanent", 2),
        alice(),
        Zone::Graveyard,
    );
    let mut dm = SplitPileChoices {
        inner: ControllerReferenceChoices {
            targets,
            accept: choose_first,
            trace: Vec::new(),
        },
        pile_size: count / 2,
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.inner.trace);
    let count_named = |ids: &[ObjectId]| {
        ids.iter()
            .filter(|id| {
                game.object(**id)
                    .is_some_and(|o| o.name.starts_with("Split permanent"))
            })
            .count()
    };
    let actual = json!({"targeted_in_hand":count_named(&game.player(alice()).unwrap().hand),"targeted_in_graveyard":count_named(&game.player(alice()).unwrap().graveyard),"targeted_in_exile":count_named(&game.exile),"untargeted_still_in_graveyard":game.object(untouched).is_some_and(|o|o.zone==Zone::Graveyard)});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn sorin_optional_sacrifice(
    definition: &CardDefinition,
    available: bool,
    accept: bool,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    if available {
        let vampire = CardDefinitionBuilder::new(CardId::new(), "Sorin sacrifice Vampire")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Vampire])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&vampire, alice(), Zone::Battlefield);
    }
    let mut dm = ControllerReferenceChoices {
        targets: vec![ironsmith::game_state::Target::Player(PlayerId::from_index(
            1,
        ))],
        accept,
        trace: Vec::new(),
    };
    let mut loyalty_before = None;
    let mut source = None;
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        let id = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|o| o.name == definition.name())
            })
            .ok_or("Sorin did not enter")?;
        source = Some(id);
        loyalty_before = Some(game.counter_count(id, CounterType::Loyalty));
        let action=compute_legal_actions(&game,alice()).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,ability_index} if *source==id&&*ability_index==1)).ok_or("second loyalty ability unavailable")?;
        dm.trace.push(json!({"stage":"legal_loyalty_action","action":format!("{action:?}"),"loyalty_before":loyalty_before,"ability":format!("{:?}",game.current_activated_ability(id,1))}));
        let mut queue = announce_chosen(&mut game, action, &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"alice_life":game.player(alice()).unwrap().life,"bob_life":game.player(PlayerId::from_index(1)).unwrap().life,
        "vampires_sacrificed":game.player(alice()).unwrap().graveyard.iter().filter(|id|game.object(**id).is_some_and(|o|o.name=="Sorin sacrifice Vampire")).count(),
        "loyalty_added":source.zip(loyalty_before).map(|(id,before)|game.counter_count(id,CounterType::Loyalty).saturating_sub(before))});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

fn synthetic_next_endstep(
    definition: &CardDefinition,
    count: usize,
    trace: &mut Vec<Value>,
) -> Result<Value, String> {
    let mut game = setup();
    for n in 0..count {
        game.create_object_from_definition(
            &creature(&format!("Synthetic exiled original {n}"), 2),
            alice(),
            Zone::Battlefield,
        );
    }
    let opponent = game.create_object_from_definition(
        &creature("Synthetic opponent creature", 2),
        PlayerId::from_index(1),
        Zone::Battlefield,
    );
    // Library top is the last element: land, creature A, land, creature B, land.
    for (n, is_creature) in [(0, false), (1, true), (2, false), (3, true), (4, false)] {
        let card = if is_creature {
            creature(&format!("Synthetic replacement {n}"), 2)
        } else {
            CardDefinitionBuilder::new(CardId::new(), &format!("Synthetic noncreature {n}"))
                .card_types(vec![CardType::Land])
                .build()
        };
        game.create_object_from_definition(&card, alice(), Zone::Library);
    }
    let mut dm = ControllerReferenceChoices {
        targets: Vec::new(),
        accept: true,
        trace: Vec::new(),
    };
    let result: Result<(), String> = (|| {
        let mut queue = cast_from_hand(&mut game, definition, alice(), &mut dm)?;
        finish_with_priority(&mut game, &mut queue, &mut dm)?;
        dm.trace.push(json!({"stage":"after_spell_before_end","delayed_triggers":format!("{:?}",game.effect_store.delayed_triggers),"library_count":game.player(alice()).unwrap().library.len()}));
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::End);
        generate_and_queue_step_triggers(&mut game, &mut queue);
        let resolved = finish_with_priority(&mut game, &mut queue, &mut dm)?;
        dm.trace.push(json!({"stage":"actual_next_end_step","resolved_stack_entries":resolved,"remaining_delayed":game.effect_store.delayed_triggers.len()}));
        Ok(())
    })();
    trace.extend(dm.trace);
    let actual = json!({"originals_exiled":game.exile.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Synthetic exiled original"))).count(),
        "replacement_creatures_in_play":game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|o|o.name.starts_with("Synthetic replacement"))).count(),
        "library_remaining":game.player(alice()).unwrap().library.len(),"opponent_creature_survives":game.object(opponent).is_some_and(|o|o.zone==Zone::Battlefield),"remaining_delayed":game.effect_store.delayed_triggers.len()});
    trace.push(json!({"partial_state":actual}));
    result?;
    Ok(actual)
}

#[test]
#[ignore = "manual audit records observed defects; report generation is not a semantics pass"]
fn report_remaining_target_zone_and_delayed_contexts() {
    let names = [
        "Settle the Wreckage",
        "Stench of Evil",
        "Split the Spoils",
        "Sorin, Imperious Bloodlord",
        "Synthetic Destiny",
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
        match name.as_str() {
            "Settle the Wreckage" => {
                for count in [0, 2] {
                    for accept in [false, true] {
                        let lands = if accept { count } else { 0 };
                        let mut trace = Vec::new();
                        record(
                            &mut rows,
                            &name,
                            json!({"actual_attackers":count,"search_accepted":accept,"target_library_basic_lands":3,"caster_is_defender":true}),
                            json!({"exiled_attackers":count,"idle_survives":true,"defender_survives":true,"basic_lands_in_play":lands,"basic_lands_tapped":lands,"target_library_remaining":3-lands}),
                            settle_actual_attackers(&definition, count, accept, &mut trace),
                            "Actual legal attack declaration, defender's paid instant cast targeting attacker controller, real optional search; nonattacking creatures preserved",
                            checksum,
                        );
                        rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                    }
                }
            }
            "Stench of Evil" => {
                for counts in [[0, 0, 0], [1, 2, 0]] {
                    for pay in [false, true] {
                        let mut trace = Vec::new();
                        record(
                            &mut rows,
                            &name,
                            json!({"plains_per_player":counts,"pay_two_for_each_destroyed_land":pay,"mana_available_for_every_payment":true}),
                            json!({"life_totals":counts.map(|n|if pay {20}else{20-n}),"resolution_mana_paid":counts.map(|n|if pay {2*n}else{0}),"plains_in_graveyards":counts,"forests_survive":3}),
                            stench_actual_plains(&definition, counts, pay, &mut trace),
                            "Actual paid normal sorcery cast with separate owners/controllers for Plains, one untargeted Forest each, sufficient mana for each optional payment; spell cost excluded from recorded resolution payments",
                            checksum,
                        );
                        rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                    }
                }
            }
            "Split the Spoils" => {
                for count in [0, 2, 5] {
                    for choose_first in [false, true] {
                        let hand = if choose_first {
                            count / 2
                        } else {
                            count - count / 2
                        };
                        let mut trace = Vec::new();
                        record(
                            &mut rows,
                            &name,
                            json!({"selected_target_count":count,"first_pile_size":count/2,"opponent_selects_first_pile":choose_first}),
                            json!({"targeted_in_hand":hand,"targeted_in_graveyard":count-hand,"targeted_in_exile":0,"untargeted_still_in_graveyard":true}),
                            split_spoils_piles(&definition, count, choose_first, &mut trace),
                            "Actual paid normal sorcery cast targeting zero, two or five owned permanent cards in graveyard; caster explicitly splits the resulting exile group, then opponent must choose a pile without becoming a target",
                            checksum,
                        );
                        rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                    }
                }
            }
            "Sorin, Imperious Bloodlord" => {
                for (available, accept) in [(false, false), (true, false), (true, true)] {
                    let sacrificed = available && accept;
                    let mut trace = Vec::new();
                    record(
                        &mut rows,
                        &name,
                        json!({"vampire_available":available,"sacrifice_accepted":accept,"ability_index":1}),
                        json!({"alice_life":if sacrificed {23}else{20},"bob_life":if sacrificed {17}else{20},"vampires_sacrificed":usize::from(sacrificed),"loyalty_added":1}),
                        sorin_optional_sacrifice(&definition, available, accept, &mut trace),
                        "Actual paid planeswalker cast then legally available second +1 loyalty activation; optional real Vampire sacrifice; reflexive damage explicitly targets Bob and full priority processing drains follow-up triggers",
                        checksum,
                    );
                    rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                }
            }
            "Synthetic Destiny" => {
                for count in [0, 1, 2] {
                    let mut trace = Vec::new();
                    record(
                        &mut rows,
                        &name,
                        json!({"controlled_creatures_exiled":count,"library_creature_count":2,"library_noncreature_count":3,"advance_to_actual_next_end_step":true}),
                        json!({"originals_exiled":count,"replacement_creatures_in_play":count,"library_remaining":5-count,"opponent_creature_survives":true,"remaining_delayed":0}),
                        synthetic_next_endstep(&definition, count, &mut trace),
                        "Actual paid instant cast and real next-end-step producer; alternating noncreature/creature library with enough creatures; opponent creature remains unaffected",
                        checksum,
                    );
                    rows.last_mut().unwrap()["execution_trace"] = json!(trace);
                }
            }
            _ => unreachable!(),
        }
    }
    write_context_report(
        "remaining-target-zone-delayed-reproductions.json",
        "Twenty actual casting, attack, loyalty and delayed-trigger scenarios across five remaining target/zone candidates",
        "Fixtures do not include control-changing lands, unpayable mana, stolen attackers, target invalidation, responses or insufficient library creatures. Split the Spoils choice traces distinguish pile construction from the opponent's selection. These checks cover the second Sorin ability only. Passing report generation does not imply semantic correctness.",
        rows,
    );
}

use ironsmith::decision::{
    GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use serde_json::{Value, json};
use std::collections::HashMap;

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for color in [ManaSymbol::Red, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
    game
}

fn cast_or_activate(
    game: &mut GameState,
    id: ObjectId,
    cast: bool,
) -> Result<(ObjectId, u32), String> {
    let stable = game.object(id).ok_or("action source missing")?.stable_id;
    game.turn.priority_player = Some(alice());
    let action = compute_legal_actions(game, alice())
        .expect("fixture has complete replacement state")
        .into_iter()
        .find(|action| match action {
            LegalAction::CastSpell { spell_id, .. } => cast && *spell_id == id,
            LegalAction::ActivateAbility { source, .. } => !cast && *source == id,
            _ => false,
        })
        .ok_or("fixture has no matching legal action")?;
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .map_err(|error| error.to_string())?;
    for _ in 0..24 {
        if let Some(result) = game
            .stack
            .iter()
            .find(|entry| {
                game.object(entry.object_id)
                    .is_some_and(|object| object.stable_id == stable)
            })
            .map(|entry| (entry.object_id, entry.mana_spent_on_activation.total()))
        {
            drain_pending_trigger_events(game, &mut queue);
            put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
                .map_err(|error| error.to_string())?;
            return Ok(result);
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::Targets(ref context)) => {
                let target = ironsmith::game_state::Target::Player(PlayerId::from_index(1));
                if context.requirements.len() != 1
                    || !context.requirements[0].legal_targets.contains(&target)
                {
                    return Err(format!(
                        "fixture cannot choose the intended opponent: {context:?}"
                    ));
                }
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::Targets(vec![target]),
                    &mut dm,
                )
                .map_err(|error| error.to_string())?
            }
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref context))
                if context.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(vec![]),
                    &mut dm,
                )
                .map_err(|error| error.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref context)
                if !matches!(context, DecisionContext::Priority(_)) =>
            {
                apply_decision_context_with_dm(game, &mut queue, &mut state, context, &mut dm)
                    .map_err(|error| error.to_string())?
            }
            ref other => return Err(format!("fixture action did not reach the stack: {other:?}")),
        };
    }
    Err("fixture exceeded action decision bound".into())
}

fn resolve_all(game: &mut GameState) -> Result<usize, String> {
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut resolved = 0;
    for _ in 0..24 {
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, &mut dm)
            .map_err(|error| error.to_string())?;
        if game.stack.is_empty() {
            return Ok(resolved);
        }
        resolve_stack_entry_with(game, &mut dm).map_err(|error| error.to_string())?;
        resolved += 1;
    }
    Err("fixture exceeded resolution bound".into())
}

fn scenario(
    definitions: &HashMap<String, CardDefinition>,
    spell: &str,
    has_tamanoa: bool,
) -> Result<Value, String> {
    let mut game = setup();
    if has_tamanoa {
        game.create_object_from_definition(&definitions["Tamanoa"], alice(), Zone::Battlefield);
    }
    let pridemate = game.create_object_from_definition(
        &definitions["Ajani's Pridemate"],
        alice(),
        Zone::Battlefield,
    );
    let source = game.create_object_from_definition(&definitions[spell], alice(), Zone::Hand);
    let before = game.player(alice()).unwrap().mana_pool.total();
    let (stack_id, _) = cast_or_activate(&mut game, source, true)?;
    let paid = before - game.player(alice()).unwrap().mana_pool.total();
    if paid != if spell == "Char" { 3 } else { 1 } {
        return Err(format!("fixture cast paid unexpected mana: {paid}"));
    }
    let entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == stack_id)
        .ok_or("fixture spell disappeared")?;
    if entry.targets
        != [ironsmith::game_state::Target::Player(PlayerId::from_index(
            1,
        ))]
    {
        return Err(format!(
            "fixture selected wrong target: {:?}",
            entry.targets
        ));
    }
    let error = resolve_all(&mut game).err();
    Ok(
        json!({"resolution_error":error, "alice_life": game.player(alice()).unwrap().life,
        "bob_life":game.player(PlayerId::from_index(1)).unwrap().life,
        "pridemate_counters":game.object(pridemate).ok_or("Pridemate unexpectedly absent")?.counters.get(&ironsmith::object::CounterType::PlusOnePlusOne).copied().unwrap_or(0)}),
    )
}

#[test]
fn simultaneous_damage_causes_one_lifegain_trigger() {
    let definitions: HashMap<_, _> = ["Char", "Lightning Bolt", "Tamanoa", "Ajani's Pridemate"]
        .into_iter()
        .map(|name| (name.to_owned(), super::load(name)))
        .collect();
    for (spell, tamanoa, alice_life, bob_life, counters) in [
        ("Lightning Bolt", true, 23, 17, 1),
        ("Char", false, 18, 16, 0),
        ("Char", true, 24, 16, 1),
    ] {
        let expected = json!({"resolution_error":null,"alice_life":alice_life,"bob_life":bob_life,"pridemate_counters":counters});
        let actual = scenario(&definitions, spell, tamanoa).unwrap();
        assert_eq!(actual, expected, "{spell}, Tamanoa present: {tamanoa}");
    }
}

#[test]
fn successive_damage_and_explicit_recipient_triggers_remain_separate() {
    for successive in [false, true] {
        let mut definitions: HashMap<_, _> = ["Char", "Tamanoa", "Ajani's Pridemate"]
            .into_iter()
            .map(|name| (name.to_owned(), super::load(name)))
            .collect();
        if successive {
            definitions.insert("Char".into(), super::compile_test_card(
                "Successive damage control", "Mana cost: {2}{R}\nType: Instant\nSuccessive damage control deals 4 damage to any target. Then Successive damage control deals 2 damage to you."));
        } else {
            definitions.insert("Tamanoa".into(), super::compile_test_card(
                "Recipient trigger control", "Mana cost: {R}{G}{W}\nType: Creature — Spirit\nPower/Toughness: 2/4\nWhenever a noncreature source you control deals damage to a player, you gain that much life."));
        }
        let actual = scenario(&definitions, "Char", true).unwrap();
        assert_eq!(
            actual,
            json!({"resolution_error":null,"alice_life":24,"bob_life":16,"pridemate_counters":2}),
            "successive instructions: {successive}"
        );
    }
}

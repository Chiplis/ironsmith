//! Child-process-only bounded audit. Run one AUDIT_NONCOMPLETION_CASE at a time.
//! Stage traces survive a timeout/abort; a passing reporter is not a verdict.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas_with, drain_pending_trigger_events,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Subtype, Zone};
use serde_json::{Value, json};
use std::io::Write;

fn emit(stage: &str, data: Value) {
    println!("AUDIT_STAGE {}", json!({"stage":stage,"data":data}));
    std::io::stdout().flush().unwrap();
}
struct Decisions {
    pay: bool,
    mode: usize,
}
impl DecisionMaker for Decisions {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        emit(
            "boolean_choice",
            json!({"description":ctx.description,"player":ctx.player.index(),"chosen":self.pay}),
        );
        self.pay
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        let chosen = if self.pay {
            20.clamp(ctx.min, ctx.max)
        } else {
            ctx.min
        };
        emit(
            "number_choice",
            json!({"description":ctx.description,"player":ctx.player.index(),"min":ctx.min,"max":ctx.max,"chosen":chosen}),
        );
        chosen
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let chosen = if ctx
            .options
            .iter()
            .any(|o| o.description.contains("13 life"))
        {
            vec![ctx.options[self.mode].index]
        } else {
            SelectFirstDecisionMaker.decide_options(game, ctx)
        };
        emit(
            "options_choice",
            json!({"description":ctx.description,"chosen":chosen,"options":ctx.options.iter().map(|o|&o.description).collect::<Vec<_>>()}),
        );
        chosen
    }
}
fn announce(
    game: &mut GameState,
    action: LegalAction,
    dm: &mut impl DecisionMaker,
) -> Result<(), String> {
    emit(
        "legal_action",
        json!({"action":format!("{action:?}"),"mana_before":game.player(PlayerId(0)).unwrap().mana_pool.total()}),
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..48 {
        if state.pending_cast.is_none()
            && state.pending_activation.is_none()
            && !game.stack.is_empty()
        {
            emit(
                "announced",
                json!({"stack":game.stack.len(),"mana_after":game.player(PlayerId(0)).unwrap().mana_pool.total()}),
            );
            return Ok(());
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            return Err(format!("announcement stalled: {progress:?}"));
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
            .map_err(|e| e.to_string())?;
    }
    Err("announcement step bound".into())
}
fn cast(
    game: &mut GameState,
    def: &CardDefinition,
    dm: &mut impl DecisionMaker,
) -> Result<ObjectId, String> {
    let source = game.create_object_from_definition(def, PlayerId(0), Zone::Hand);
    emit("before_cast_discovery", json!({"source":source.0}));
    let action = compute_legal_actions(game, PlayerId(0)).expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..} if *spell_id==source))
        .ok_or("no legal cast")?;
    announce(game, action, dm)?;
    emit("before_spell_resolution", json!({"source":source.0}));
    resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    emit(
        "spell_resolved",
        json!({"battlefield":game.battlefield.len(),"stack":game.stack.len()}),
    );
    Ok(game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| o.name == def.card.name))
        .unwrap_or(source))
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) -> Result<(), String> {
    let mut queue = TriggerQueue::new();
    for _ in 0..20 {
        emit(
            "before_sba_and_triggers",
            json!({"life":game.players.iter().map(|p|p.life).collect::<Vec<_>>(),"stack":game.stack.len()}),
        );
        check_and_apply_sbas_with(game, &mut queue, dm);
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, dm).map_err(|e| e.to_string())?;
        if game.stack.is_empty() {
            return Ok(());
        }
        emit(
            "before_trigger_resolution",
            json!({"stack":game.stack.len()}),
        );
        resolve_stack_entry_with(game, dm).map_err(|e| e.to_string())?;
    }
    Err("resolution step bound".into())
}
fn scenario(def: &CardDefinition, case: &str) -> Result<Value, String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(71757432704846);
    game.turn.turn_number = 3;
    game.turn.active_player = PlayerId(0);
    game.turn.priority_player = Some(PlayerId(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ] {
        game.player_mut(PlayerId(0))
            .unwrap()
            .mana_pool
            .add(symbol, 10);
    }
    let soldier = CardDefinitionBuilder::new(CardId::new(), "Noncompletion Soldier")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    let insect = CardDefinitionBuilder::new(CardId::new(), "Noncompletion Insect")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
        .build();
    let mut dm = Decisions {
        pay: case == "plague_pay20",
        mode: usize::from(case.ends_with("mode1")),
    };
    if case.starts_with("goddric") {
        let prior = usize::from(case.ends_with("prior1"));
        for _ in 0..prior {
            let p = game.create_object_from_definition(&soldier, PlayerId(0), Zone::Hand);
            game.move_object_by_effect(p, Zone::Battlefield)
                .ok_or("prior entry failed")?;
        }
        settle(&mut game, &mut dm)?;
        emit("earlier_entries", json!({"count":prior}));
        let source = cast(&mut game, def, &mut dm)?;
        settle(&mut game, &mut dm)?;
        let chars = game
            .calculated_characteristics(source)
            .ok_or("source characteristics missing")?;
        return Ok(
            json!({"power":chars.power,"toughness":chars.toughness,"stack":game.stack.len()}),
        );
    }
    if case.starts_with("grist") {
        let insect_on_top = case == "grist_insect_then_soldier";
        for d in if insect_on_top {
            [&soldier, &insect]
        } else {
            [&insect, &soldier]
        } {
            game.create_object_from_definition(d, PlayerId(0), Zone::Library);
        }
        let source = cast(&mut game, def, &mut dm)?;
        settle(&mut game, &mut dm)?;
        let ability = def
            .abilities
            .iter()
            .position(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .ok_or("no activated ability")?;
        let action=compute_legal_actions(&game,PlayerId(0)).expect("fixture has complete replacement state").into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index,..} if *s==source && *ability_index==ability)).ok_or("Grist +1 not legal")?;
        emit(
            "known_library",
            json!({"top_to_bottom":game.player(PlayerId(0)).unwrap().library.iter().rev().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}),
        );
        announce(&mut game, action, &mut dm)?;
        emit(
            "before_grist_plus_one_resolution",
            json!({"library":game.player(PlayerId(0)).unwrap().library.len()}),
        );
        resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
        let tokens = game
            .battlefield
            .iter()
            .filter(|id| game.object(**id).is_some_and(|o| o.name == "Insect"))
            .count();
        return Ok(
            json!({"insect_tokens":tokens,"library_remaining":game.player(PlayerId(0)).unwrap().library.len(),"stack":game.stack.len()}),
        );
    }
    if case.starts_with("plague") {
        cast(&mut game, def, &mut dm)?;
        settle(&mut game, &mut dm)?;
    } else {
        cast(&mut game, def, &mut dm)?;
        settle(&mut game, &mut dm)?;
        let life = if case.contains("life13") { 13 } else { 20 };
        for p in &mut game.players {
            p.life = life;
        }
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::Step::Upkeep);
        let event =
            ironsmith::triggers::generate_step_trigger_events(&game).ok_or("no upkeep event")?;
        let mut queue = TriggerQueue::new();
        for entry in ironsmith::triggers::check_triggers(&game, &event) {
            queue.add(entry);
        }
        emit(
            "before_upkeep_trigger_placement",
            json!({"life":life,"mode":dm.mode,"trigger_count":queue.entries.len()}),
        );
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).map_err(|e| e.to_string())?;
        emit(
            "before_upkeep_resolution",
            json!({"stack":game.stack.len()}),
        );
        resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
        settle(&mut game, &mut dm)?;
    }
    Ok(
        json!({"life":game.players.iter().map(|p|p.life).collect::<Vec<_>>(),"players_in_game":game.players_in_game(),"stack":game.stack.len()}),
    )
}
#[test]
#[ignore = "run each scenario in a timeout-bounded child process"]
fn report_noncompletion_case() {
    let case = std::env::var("AUDIT_NONCOMPLETION_CASE").unwrap();
    let name = if case.starts_with("goddric") {
        "Goddric, Cloaked Reveler"
    } else if case.starts_with("grist") {
        "Grist, the Hunger Tide"
    } else if case.starts_with("plague") {
        "Plague of Vermin"
    } else {
        "Triskaidekaphobia"
    };
    let inventory = std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap();
    let data: Value = serde_json::from_slice(&std::fs::read(inventory).unwrap()).unwrap();
    let p = data["cards"]
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
    emit(
        "strict_compiled",
        json!({"card":name,"case":case,"artifact_checksum":artifact.payload_checksum}),
    );
    let result = scenario(&def, &case);
    emit(
        "finished",
        match result {
            Ok(actual) => json!({"actual":actual}),
            Err(error) => json!({"fixture_or_execution_error":error}),
        },
    );
}

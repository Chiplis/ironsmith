//! Canonical small legal scenarios for nested panic candidates, one per isolated child.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext,
};
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
    count: usize,
}
impl DecisionMaker for Decisions {
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let wanted = self.count.max(ctx.min).min(ctx.max.unwrap_or(usize::MAX));
        let selected: Vec<_> = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .take(wanted)
            .map(|c| c.id)
            .collect();
        emit(
            "objects_choice",
            json!({"context":format!("{ctx:?}"),"selected":selected.iter().map(|id|id.0).collect::<Vec<_>>() }),
        );
        selected
    }

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
        let chosen = SelectFirstDecisionMaker.decide_options(game, ctx);
        emit(
            "options_choice",
            json!({"description":ctx.description,"chosen":chosen,"options":ctx.options.iter().map(|o|&o.description).collect::<Vec<_>>()}),
        );
        chosen
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Decisions) -> Result<(), String> {
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
        if state.pending_cast.as_ref().is_some_and(|p| {
            matches!(
                p.stage,
                ironsmith::game_loop::CastStage::ChoosingOptionalCosts
            )
        }) {
            let selected = if dm.pay { vec![(0, 1)] } else { vec![] };
            emit("optional_cost_choice", json!({"selected":selected}));
            progress = apply_priority_response_with_dm(
                game,
                &mut queue,
                &mut state,
                &PriorityResponse::OptionalCosts(selected),
                dm,
            )
            .map_err(|e| e.to_string())?;
        } else {
            progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm)
                .map_err(|e| e.to_string())?;
        }
    }
    Err("announcement step bound".into())
}
fn cast(
    game: &mut GameState,
    def: &CardDefinition,
    dm: &mut Decisions,
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

fn scenario(
    def: &CardDefinition,
    back: Option<&CardDefinition>,
    case: &str,
) -> Result<Value, String> {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(71757432704847);
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
    let mut dm = Decisions {
        pay: case.ends_with("accept"),
        count: if case.ends_with("two") { 2 } else { 1 },
    };
    let filler = CardDefinitionBuilder::new(CardId::new(), "Nested panic library card")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dinosaur])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 3))
        .build();
    for _ in 0..10 {
        game.create_object_from_definition(&filler, PlayerId(0), Zone::Library);
    }
    if let Some(back) = back {
        game.register_linked_face_definition(def);
        game.register_linked_face_definition(back);
    }
    let source = cast(&mut game, def, &mut dm)?;
    settle(&mut game, &mut dm)?;
    if let Some(back) = back {
        let mut materials = Vec::new();
        for power in [3, 5].into_iter().take(dm.count) {
            let material =
                CardDefinitionBuilder::new(CardId::new(), format!("Craft material power {power}"))
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![Subtype::Dinosaur])
                    .power_toughness(ironsmith::PowerToughness::fixed(power, power))
                    .build();
            materials.push(game.create_object_from_definition(
                &material,
                PlayerId(0),
                Zone::Graveyard,
            ));
        }
        emit(
            "craft_materials_ready",
            json!({"power_total":if dm.count==2{8}else{3},"ids":materials.iter().map(|id|id.0).collect::<Vec<_>>(),"front":game.object(source).unwrap().name.to_string()}),
        );
        let ability = def
            .abilities
            .iter()
            .position(|a| matches!(&a.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .ok_or("no craft ability")?;
        let actions = compute_legal_actions(&game, PlayerId(0)).expect("fixture has complete replacement state");
        emit(
            "craft_legal_actions",
            json!({"actions":format!("{actions:?}"),"ability":ability}),
        );
        let action=actions.into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:s,ability_index,..} if *s==source&&*ability_index==ability)).ok_or("craft activation not offered")?;
        announce(&mut game, action, &mut dm)?;
        emit(
            "craft_cost_paid",
            json!({"exile":game.exile.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>(),"stack":game.stack.len()}),
        );
        resolve_stack_entry_with(&mut game, &mut dm).map_err(|e| e.to_string())?;
        emit(
            "craft_resolved",
            json!({"battlefield":game.battlefield.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>()}),
        );
        settle(&mut game, &mut dm)?;
        let transformed = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).is_some_and(|o| o.name == back.card.name))
            .ok_or("back face absent after craft")?;
        let chars = game
            .calculated_characteristics(transformed)
            .ok_or("back characteristics absent")?;
        return Ok(
            json!({"name":back.card.name,"power":chars.power,"toughness":chars.toughness,"stack":game.stack.len()}),
        );
    }
    Ok(
        json!({"stack":game.stack.len(),"hand":game.player(PlayerId(0)).unwrap().hand.len(),"life":game.players.iter().map(|p|p.life).collect::<Vec<_>>()}),
    )
}
fn compile(data: &Value, name: &str) -> Result<CardDefinition, String> {
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
    .map_err(|e| e.to_string())?;
    emit(
        "strict_compiled",
        json!({"card":name,"artifact_checksum":artifact.payload_checksum,"definition":artifact.payload.definition}),
    );
    Ok(def)
}
#[test]
#[ignore = "isolated audit observation, not a gameplay correctness assertion"]
fn report_nested_panic_case() {
    let case = std::env::var("AUDIT_NESTED_PANIC_CASE").unwrap();
    let name = if case.starts_with("katara") {
        "Katara, Seeking Revenge"
    } else if case.starts_with("ruinous") {
        "Ruinous Waterbending"
    } else if case.starts_with("revival") {
        "Spirit Water Revival"
    } else if case.starts_with("raptor") {
        "Saheeli's Lattice"
    } else {
        "Altar of the Wretched"
    };
    let data: Value = serde_json::from_slice(
        &std::fs::read(std::env::var("AUDIT_RUNTIME_INVENTORY").unwrap()).unwrap(),
    )
    .unwrap();
    let mut def = match compile(&data, name) {
        Ok(d) => d,
        Err(e) => {
            emit("finished", json!({"compile_error":e,"card":name}));
            return;
        }
    };
    let mut back = if case.starts_with("raptor") {
        Some(compile(&data, "Mastercraft Raptor").unwrap())
    } else if case.starts_with("bonemass") {
        Some(compile(&data, "Wretched Bonemass").unwrap())
    } else {
        None
    };
    if let Some(back) = back.as_mut() {
        def.card.other_face = Some(back.card.id);
        def.card.other_face_name = Some(back.card.name.clone());
        def.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        def.card.transforming_dfc = true;
        back.card.other_face = Some(def.card.id);
        back.card.other_face_name = Some(def.card.name.clone());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        back.card.transforming_dfc = true;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scenario(&def, back.as_ref(), &case)
    }));
    emit(
        "finished",
        match result {
            Ok(Ok(actual)) => json!({"actual":actual}),
            Ok(Err(error)) => json!({"fixture_or_execution_error":error}),
            Err(p) => {
                json!({"panic":p.downcast_ref::<String>().cloned().or_else(||p.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or("nonstring panic".into())})
            }
        },
    );
}

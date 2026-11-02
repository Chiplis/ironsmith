//! The generic lifetime used by cast costs that exile a resource until the
//! spell fails to enter, or the permanent it becomes subsequently leaves.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::EffectContext;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

// Resolve through the production stack owner, which publishes completed
// events and processes exile-until duration ends before granting priority.
fn resolve_instruction(game: &mut GameState, effect: &Effect, ctx: &mut EffectContext) -> Result<(), String> {
    game.push_to_stack(ironsmith::game_state::StackEntry::ability(ctx.source, ctx.controller, vec![effect.clone()]));
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut *ctx.decision_maker).map(|_| ()).map_err(|error| format!("{error:?}"))
}
fn definition(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap()
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn cast(game: &mut GameState) -> ObjectId {
    let card = definition(
        "Cast exile source",
        "Mana cost: {0}\nType: Artifact Creature — Construct\nPower/Toughness: 2/2",
    );
    let source = game.create_object_from_definition(&card, A, Zone::Hand);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .unwrap();
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            return game.stack[0].object_id;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("cast did not finish");
}
// This explicit test player channel authorizes required return/entry choices.
// SelectFirstDecisionMaker intentionally reports no real player channel.
struct PlayerChoices;
impl ironsmith::decision::DecisionMaker for PlayerChoices {}
fn drain(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut PlayerChoices,
    )
    .unwrap();
    assert!(queue.entries.is_empty());
}
fn exile(
    game: &mut GameState,
    source: ObjectId,
    roundtrip: bool,
) -> (ObjectId, ironsmith::ids::StableId) {
    let victim = game.create_object_from_definition(
        &definition(
            "Exiled resource",
            "Type: Creature — Fox\nPower/Toughness: 1/1",
        ),
        B,
        Zone::Battlefield,
    );
    let stable = game.object(victim).unwrap().stable_id;
    let effect = ironsmith_core::ExileUntilEffect::new(
        ChooseSpec::SpecificObject(victim),
        ironsmith_core::ExileUntilDuration::SourceLeavesStackOrBattlefield,
    );
    let effect = if roundtrip {
        serde_json::from_str(&serde_json::to_string(&effect).unwrap()).unwrap()
    } else {
        effect
    };
    resolve_instruction(
        game,
        &Effect::new(effect),
        &mut EffectContext::new_default(source, A),
    )
    .unwrap();
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    (exiled, stable)
}
#[test]
fn cast_exile_survives_normal_stack_to_battlefield_and_returns_on_resulting_permanent_exit() {
    for roundtrip in [false, true] {
        let mut game = setup();
        let spell = cast(&mut game);
        let source_stable = game.object(spell).unwrap().stable_id;
        let (exiled, stable) = exile(&mut game, spell, roundtrip);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        drain(&mut game);
        let permanent = game.find_object_by_stable_id(source_stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.object(exiled).unwrap().zone,
            Zone::Exile,
            "normal resolution must not end duration"
        );
        assert!(game.get_exiled_with_source_links(permanent).contains(&exiled));
        resolve_instruction(
            &mut game,
            &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(permanent),
                Zone::Hand,
                false,
            )),
            &mut EffectContext::new_default(permanent, A),
        )
        .unwrap();
        drain(&mut game);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.current_controller(returned), Some(B));
    }
}
#[test]
fn countering_or_bouncing_the_source_spell_ends_the_duration() {
    for roundtrip in [false, true] {
        for counter in [false, true] {
            let mut game = setup();
            let spell = cast(&mut game);
            let (_, stable) = exile(&mut game, spell, roundtrip);
            let effect = if counter {
                Effect::counter(ChooseSpec::SpecificObject(spell))
            } else {
                Effect::new(ironsmith::effects::MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(spell),
                    Zone::Hand,
                    false,
                ))
            };
            resolve_instruction(
                &mut game,
                &effect,
                &mut EffectContext::new_default(spell, A),
            )
            .unwrap();
            drain(&mut game);
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        }
    }
}
#[test]
fn resource_departure_from_exile_breaks_its_exact_incarnation_link() {
    let mut game = setup();
    let spell = cast(&mut game);
    let (exiled, stable) = exile(&mut game, spell, false);
    resolve_instruction(
        &mut game,
        &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(exiled),
            Zone::Graveyard,
            false,
        )),
        &mut EffectContext::new_default(spell, A),
    )
    .unwrap();
    resolve_instruction(
        &mut game,
        &Effect::counter(ChooseSpec::SpecificObject(spell)),
        &mut EffectContext::new_default(spell, A),
    )
    .unwrap();
    drain(&mut game);
    let card = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(card).unwrap().zone, Zone::Graveyard);
}

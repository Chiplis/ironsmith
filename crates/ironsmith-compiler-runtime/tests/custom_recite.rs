//! Recite composes a graveyard price, selected-method battlefield redirect,
//! and a real conditional entry trigger. Parent registers and runs this target.
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
const A: PlayerId = PlayerId(0);
const RULES: &str = "You may cast this card from your graveyard for {1}{B}{G}. If the permanent this spell becomes would leave the battlefield, exile it instead.\nWhen this permanent enters, if its recite cost was paid, exile it.";
fn compile(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    name,
                )
                .with_custom_keyword("recite"),
                text,
                ironsmith_compiler::CompilePolicy {
                    allow_unsupported: false,
                },
            )
        })
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    assert!(
        compiled
            .definition
            .attribute_custom_keyword_payment("recite")
    );
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct =
        ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone())
            .unwrap();
    let artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(direct.card.id.0),
            name: direct.card.name.clone(),
            face_name: None,
            other_face: None,
            linked_face_layout: None,
        },
        CompiledCardPayload {
            definition: wire,
            canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
            ability_labels: ironsmith_text::ability_surface_texts(&direct),
        },
        "recite-composition-test",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn definitions() -> [CardDefinition; 2] {
    compile(
        "Recite witness",
        &format!(
            "Mana cost: {{2}}{{B}}{{G}}\nType: Creature — Elemental\nPower/Toughness: 2/2\nWhen this creature enters, you gain 1 life.\n{RULES}"
        ),
    )
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn fund(game: &mut GameState, generic: u32) {
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    pool.add(ManaSymbol::Black, 1);
    pool.add(ManaSymbol::Green, 1);
    pool.add(ManaSymbol::Colorless, generic);
}
fn cast(game: &mut GameState, definition: &CardDefinition, recite: bool) -> ObjectId {
    let source = game.create_object_from_definition(
        definition,
        A,
        if recite { Zone::Graveyard } else { Zone::Hand },
    );
    fund(game, if recite { 1 } else { 2 });
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
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
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
fn move_effect(game: &mut GameState, source: ObjectId, destination: Zone) -> ObjectId {
    let stable = game.object(source).unwrap().stable_id;
    execute_effect(
        game,
        &Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(source),
            destination,
            false,
        )),
        &mut EffectContext::new_default(source, A),
    )
    .unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
fn entry(game: &mut GameState, spell: ObjectId) -> ObjectId {
    let stable = game.object(spell).unwrap().stable_id;
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(
        game.object(permanent).unwrap().zone,
        Zone::Battlefield,
        "entry exile must use the stack"
    );
    permanent
}
fn queue(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(
        game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
}
#[test]
fn recite_entry_is_a_real_trigger_and_ordinary_entry_keeps_the_permanent() {
    for definition in definitions() {
        for recite in [false, true] {
            let mut game = setup();
            let spell = cast(&mut game, &definition, recite);
            assert_eq!(
                game.object(spell)
                    .unwrap()
                    .optional_costs_paid
                    .paid_custom_keyword("recite"),
                recite
            );
            let permanent = entry(&mut game, spell);
            let stable = game.object(permanent).unwrap().stable_id;
            queue(&mut game);
            assert_eq!(game.stack.len(), if recite { 2 } else { 1 });
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry_with(
                    &mut game,
                    &mut SelectFirstDecisionMaker,
                )
                .unwrap();
            }
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(arrival).unwrap().zone,
                if recite {
                    Zone::Exile
                } else {
                    Zone::Battlefield
                }
            );
            assert_eq!(
                game.player(A).unwrap().life,
                21,
                "own entry trigger survives exile of its source"
            );
        }
    }
}
#[test]
fn recite_redirects_every_battlefield_exit_before_entry_trigger_but_never_stack_departures() {
    for definition in definitions() {
        let mut base = setup();
        let spell = cast(&mut base, &definition, true);
        let stable = base.object(spell).unwrap().stable_id;
        let mut countered = base.clone();
        execute_effect(
            &mut countered,
            &Effect::counter(ChooseSpec::SpecificObject(spell)),
            &mut EffectContext::new_default(spell, A),
        )
        .unwrap();
        let grave = countered.find_object_by_stable_id(stable).unwrap();
        assert_eq!(countered.object(grave).unwrap().zone, Zone::Graveyard);
        for destination in [Zone::Hand, Zone::Graveyard, Zone::Library] {
            let mut stack_exit = base.clone();
            let arrival = move_effect(&mut stack_exit, spell, destination);
            assert_eq!(stack_exit.object(arrival).unwrap().zone, destination);
        }
        let permanent = entry(&mut base, spell);
        for destination in [Zone::Hand, Zone::Graveyard, Zone::Library, Zone::Exile] {
            let mut interrupted = base.clone();
            let exiled = move_effect(&mut interrupted, permanent, destination);
            assert_eq!(interrupted.object(exiled).unwrap().zone, Zone::Exile);
            let hand = move_effect(&mut interrupted, exiled, Zone::Hand);
            let grave = move_effect(&mut interrupted, hand, Zone::Graveyard);
            assert_eq!(
                interrupted.object(grave).unwrap().zone,
                Zone::Graveyard,
                "new incarnation discards selected method redirect"
            );
        }
    }
}
#[test]
fn source_recite_variants_compile_with_their_complete_original_cards() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-middle-keywords.json"
    ))
    .unwrap();
    for row in fixture["mechanics"].as_array().unwrap() {
        if row["keyword"] != "Recite" {
            continue;
        }
        for card in row["representative_cards"].as_array().unwrap() {
            compile(
                card["name"].as_str().unwrap(),
                card["executable_text"].as_str().unwrap(),
            );
        }
    }
}
#[test]
fn recite_entry_trigger_cannot_exile_a_later_incarnation_of_its_source() {
    for definition in definitions() {
        let mut game = setup();
        let spell = cast(&mut game, &definition, true);
        let permanent = entry(&mut game, spell);
        queue(&mut game);
        assert_eq!(game.stack.len(), 2);
        let exiled = move_effect(&mut game, permanent, Zone::Hand);
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        let hand = move_effect(&mut game, exiled, Zone::Hand);
        let returned = move_effect(&mut game, hand, Zone::Battlefield);
        assert!(
            !game
                .object(returned)
                .unwrap()
                .optional_costs_paid
                .paid_custom_keyword("recite")
        );
        queue(&mut game);
        assert_eq!(
            game.stack.len(),
            3,
            "new incarnation adds ordinary ETB trigger only"
        );
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
        }
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().life, 22);
    }
}

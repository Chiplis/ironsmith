//! Printed cost exile families survive casting and exclude unrelated acquisitions.
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::effects::EffectContext;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const RULES: &str = "As an additional cost to cast this spell, you may exile a nonenchantment creature you control until this spell leaves the stack without entering the battlefield or the permanent it becomes leaves the battlefield.\nThis spell costs {X} less to cast, where X is the total mana value of creatures exiled this way.";

// Resolve through the production stack owner, which publishes completed
// events and processes exile-until duration ends before granting priority.
fn resolve_instruction(game: &mut GameState, effect: &Effect, ctx: &mut EffectContext) -> Result<(), String> {
    game.push_to_stack(ironsmith::game_state::StackEntry::ability(ctx.source, ctx.controller, vec![effect.clone()]));
    ironsmith::game_loop::resolve_stack_entry_with(game, &mut *ctx.decision_maker).map(|_| ()).map_err(|error| format!("{error:?}"))
}
fn compile(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            ironsmith_compiler::CompilerFacade::new().compile_definition(
                ironsmith_compiler::CardDefinitionBuilder::new(
                    ironsmith_compiler::ids::CardId::new(),
                    "Exile cost source",
                )
                .with_custom_keyword("enspirit"),
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
            .attribute_custom_keyword_payment("enspirit")
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
        "enspirit-cost-composition",
        text.as_bytes(),
    );
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn routes() -> [CardDefinition; 2] {
    compile(&format!(
        "Mana cost: {{4}}{{U}}{{B}}\nType: Artifact\n{RULES}"
    ))
}
fn body(game: &mut GameState, owner: PlayerId, mana: u32, enchantment: bool) -> ObjectId {
    let card = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Cost resource",
        &format!(
            "Mana cost: {{{mana}}}\nType: {}Creature — Fox\nPower/Toughness: 1/1",
            if enchantment { "Enchantment " } else { "" }
        ),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&card, owner, Zone::Battlefield)
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn fund(game: &mut GameState, generic: u32) {
    let pool = &mut game.player_mut(A).unwrap().mana_pool;
    pool.add(ManaSymbol::Blue, 1);
    pool.add(ManaSymbol::Black, 1);
    pool.add(ManaSymbol::Colorless, generic);
}
struct Choices {
    pay: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.pay
    }
    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            return ctx
                .options
                .iter()
                .filter(|o| self.pay && o.legal)
                .map(|o| o.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}
fn announce(game: &mut GameState, source: ObjectId, pay: bool) -> ObjectId {
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source))
        .expect("cast affordable with selected resource");
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = Choices { pay };
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
fn drain(game: &mut GameState) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(
        game,
        &mut queue,
        &mut Choices { pay: false },
    )
    .unwrap();
    assert!(queue.entries.is_empty());
}

fn activate(game: &mut GameState, source: ObjectId, ordinal: usize) {
    let index = game.object(source).unwrap().abilities.iter().enumerate()
        .filter(|(_, ability)| matches!(ability.kind, ironsmith::AbilityKind::Activated(_)))
        .nth(ordinal).map(|(index, _)| index).expect("printed activated ordinal exists");
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index }
            if *id == source && *ability_index == index)
        })
        .expect("printed activation legal");
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
            ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
            drain(game);
            return;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
    }
    panic!("activation did not finish");
}
#[test]
fn printed_cast_cost_and_printed_exile_ability_share_family_but_unrelated_exile_does_not() {
    let rules = format!(
        "Mana cost: {{4}}{{U}}{{B}}\nType: Artifact\n{RULES}\n{{0}}: Exile target creature you control.\n{{0}}: Return all creature cards exiled with this permanent to the battlefield."
    );
    for definition in compile(&rules) {
        let mut game = setup();
        fund(&mut game, 0);
        let cost_creature = body(&mut game, A, 4, false);
        let cost_stable = game.object(cost_creature).unwrap().stable_id;
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source_stable = game.object(source).unwrap().stable_id;
        announce(&mut game, source, true);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
            .unwrap();
        drain(&mut game);
        let source = game.find_object_by_stable_id(source_stable).unwrap();
        let unrelated = body(&mut game, A, 1, false);
        let unrelated_stable = game.object(unrelated).unwrap().stable_id;
        resolve_instruction(
            &mut game,
            &Effect::exile(ChooseSpec::SpecificObject(unrelated)),
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        drain(&mut game);
        let printed = body(&mut game, A, 2, false);
        let printed_stable = game.object(printed).unwrap().stable_id;
        activate(&mut game, source, 0);
        assert_eq!(
            game.object(game.find_object_by_stable_id(printed_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
        activate(&mut game, source, 1);
        for stable in [cost_stable, printed_stable] {
            assert_eq!(
                game.object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Battlefield
            );
        }
        assert_eq!(
            game.object(game.find_object_by_stable_id(unrelated_stable).unwrap())
                .unwrap()
                .zone,
            Zone::Exile
        );
    }
}

//! UNVALIDATED source-authored scenarios. Do not execute during the source-first campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::Color;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{ColorsContext, SelectOptionsContext};
use ironsmith::effects::EffectContext;
use ironsmith::effects::{AddManaEffect, EffectExecutor};
use ironsmith::events::{ManaAddedEvent, mana::ManaProductionProvenance};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{
    ManaPaymentRequest, check_mana_payment, execute_mana_payment_plan, plan_first_mana_payment,
};
use ironsmith::target::PlayerFilter;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/mana_output_replacements.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> Vec<CardDefinition> {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    vec![direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn card(game: &mut GameState, owner: PlayerId, text: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition("Mana fixture", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn land(game: &mut GameState, owner: PlayerId, types: &str) -> ObjectId {
    card(
        game,
        owner,
        &format!("Type: {types}\n{{T}}: Add {{G}}{{G}}."),
        Zone::Battlefield,
    )
}
struct Choices {
    color: Color,
    pause: bool,
    pending: bool,
    color_players: Vec<PlayerId>,
    option_players: Vec<PlayerId>,
    accept: bool,
}
impl Choices {
    fn new(color: Color) -> Self {
        Self {
            color,
            pause: false,
            pending: false,
            color_players: Vec::new(),
            option_players: Vec::new(),
            accept: true,
        }
    }
}
impl DecisionMaker for Choices {
    fn decide_colors(&mut self, _: &GameState, context: &ColorsContext) -> Vec<Color> {
        self.color_players.push(context.player);
        assert_eq!(context.count, 1);
        assert!(
            context
                .available_colors
                .as_ref()
                .is_none_or(|colors| colors.contains(&self.color))
        );
        if self.pause {
            self.pending = true;
            Vec::new()
        } else {
            vec![self.color]
        }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        let label = format!("{:?}", self.color);
        if context
            .options
            .iter()
            .any(|option| option.description == label)
        {
            self.option_players.push(context.player);
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| option.description == label)
                    .unwrap()
                    .index,
            ];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        context: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.accept && context.can_accept
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext,
    ) -> ironsmith::mana_payment::ManaPaymentResponse {
        SelectFirstDecisionMaker.decide_mana_payment(game, context)
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_targets(
        &mut self,
        game: &GameState,
        context: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        SelectFirstDecisionMaker.decide_targets(game, context)
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}
fn produce(
    game: &mut GameState,
    source: ObjectId,
    actor: PlayerId,
    recipient: PlayerId,
    mana: Vec<ManaSymbol>,
    tapped: bool,
    choices: &mut Choices,
) -> Vec<ManaSymbol> {
    let mut ctx =
        EffectContext::new(source, actor, choices).with_mana_production_provenance(if tapped {
            ManaProductionProvenance::TappedSourceForMana
        } else {
            ManaProductionProvenance::Unknown
        });
    let outcome = AddManaEffect::new(mana, PlayerFilter::Specific(recipient))
        .execute(game, &mut ctx)
        .unwrap();
    outcome
        .events
        .iter()
        .find_map(|event| event.downcast::<ManaAddedEvent>())
        .map(|event| event.mana.clone())
        .unwrap_or_default()
}
fn settle(game: &mut GameState, choices: &mut Choices) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..30 {
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
        if game.stack_is_empty() {
            return;
        }
        ironsmith::game_loop::resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("bounded mana scenario did not settle");
}
fn run_upkeep(game: &mut GameState, active: PlayerId, choices: &mut Choices) {
    game.turn.active_player = active;
    game.turn.phase = ironsmith::Phase::Beginning;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::turn_runner::TurnRunner::from_state_for_sync(
        ironsmith::turn_runner::TurnState::Upkeep,
    )
    .advance(game, &mut queue)
    .unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    settle(game, choices);
}
fn pay_request(game: &GameState, source: ObjectId, symbols: Vec<ManaSymbol>) -> ManaPaymentRequest {
    ManaPaymentRequest::new(
        A,
        source,
        ironsmith::costs::PaymentReason::Other,
        ManaCost::from_pips(symbols.into_iter().map(|symbol| vec![symbol]).collect()),
    )
    .with_spend_policy(game.mana_spend_policy(A, Some(source)))
}
#[test]
fn eleven_complete_frozen_bodies_have_strict_direct_and_artifact_gates() {
    for row in rows() {
        assert_eq!(definitions(row["name"].as_str().unwrap()).len(), 2);
    }
}
#[test]
fn static_type_rewriting_preserves_count_but_contamination_replaces_type_and_count() {
    for (name, output, count) in [
        ("Contamination", ManaSymbol::Black, 1),
        ("Infernal Darkness", ManaSymbol::Black, 3),
        ("Ritual of Subdual", ManaSymbol::Colorless, 3),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let source = land(&mut game, B, "Land");
            let original = vec![ManaSymbol::Green, ManaSymbol::Blue, ManaSymbol::Colorless];
            let mut choices = Choices::new(Color::Red);
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    B,
                    B,
                    original.clone(),
                    true,
                    &mut choices
                ),
                vec![output; count]
            );
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    B,
                    B,
                    original.clone(),
                    false,
                    &mut choices
                ),
                original
            );
            game.phase_out(host);
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    B,
                    B,
                    original.clone(),
                    true,
                    &mut choices
                ),
                original
            );
            game.phase_in(host);
            game.move_object_by_effect(host, Zone::Exile).unwrap();
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    B,
                    B,
                    original.clone(),
                    true,
                    &mut choices
                ),
                original
            );
        }
    }
}
#[test]
fn pulse_chooses_one_color_for_the_whole_production_and_binds_its_controller() {
    for definition in definitions("Pulse of Llanowar") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let basic = land(&mut game, A, "Basic Land — Forest");
        let nonbasic = land(&mut game, A, "Land");
        let foreign = land(&mut game, B, "Basic Land — Forest");
        let mut choices = Choices::new(Color::Blue);
        assert_eq!(
            produce(
                &mut game,
                basic,
                A,
                B,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Blue; 2]
        );
        assert_eq!(
            choices.color_players,
            vec![A],
            "the replacement controller chooses even if another player receives the mana"
        );
        for source in [nonbasic, foreign] {
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    A,
                    A,
                    vec![ManaSymbol::Green; 2],
                    true,
                    &mut choices
                ),
                vec![ManaSymbol::Green; 2]
            );
        }
        game.set_current_controller(host, B).unwrap();
        assert_eq!(
            produce(
                &mut game,
                foreign,
                B,
                B,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Blue; 2]
        );
        assert_eq!(choices.color_players.last(), Some(&B));
    }
}
#[test]
fn dual_typed_lands_select_one_mapping_for_each_whole_production() {
    for name in ["Naked Singularity", "Reality Twist"] {
        for definition in definitions(name) {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let dual = land(&mut game, B, "Land — Plains Forest");
            for color in [Color::Red, Color::Black] {
                let mut choices = Choices::new(color);
                assert_eq!(
                    produce(
                        &mut game,
                        dual,
                        B,
                        B,
                        vec![ManaSymbol::White, ManaSymbol::Green],
                        true,
                        &mut choices
                    ),
                    vec![ManaSymbol::from_color(color); 2]
                );
                assert_eq!(
                    choices.color_players,
                    vec![B],
                    "the player producing mana selects the matching land-type rewrite"
                );
            }
            let untyped = land(&mut game, B, "Land");
            assert_eq!(
                produce(
                    &mut game,
                    untyped,
                    B,
                    B,
                    vec![ManaSymbol::Green; 2],
                    true,
                    &mut Choices::new(Color::Blue)
                ),
                vec![ManaSymbol::Green; 2]
            );
        }
    }
}
#[test]
fn planner_reachability_and_paid_execution_share_exact_output_and_choice_witnesses() {
    for (name, payment, succeeds) in [
        ("Contamination", vec![ManaSymbol::Black], true),
        ("Contamination", vec![ManaSymbol::Black; 2], false),
        ("Infernal Darkness", vec![ManaSymbol::Black; 2], true),
        ("Pulse of Llanowar", vec![ManaSymbol::Blue; 2], true),
        ("Pulse of Llanowar", vec![ManaSymbol::Colorless], false),
    ] {
        for definition in definitions(name) {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let source = land(&mut game, A, "Basic Land — Forest");
            let request = pay_request(&game, source, payment.clone());
            let ids = game.next_object_id_counter();
            assert_eq!(
                check_mana_payment(&game, &request).is_ok(),
                succeeds,
                "{name}: {payment:?}"
            );
            assert!(!game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(
                game.next_object_id_counter(),
                ids,
                "queries cannot allocate actual-game identities"
            );
            if succeeds {
                let plan = plan_first_mana_payment(&game, &request).unwrap();
                execute_mana_payment_plan(
                    &mut game,
                    &request,
                    &plan,
                    &mut Choices::new(Color::Red),
                )
                .unwrap();
                assert!(game.is_tapped(source));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}
#[test]
fn a_pending_replacement_color_credits_nothing_and_replay_produces_one_complete_event() {
    for definition in definitions("Pulse of Llanowar") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = land(&mut game, A, "Basic Land — Forest");
        let ids = game.next_object_id_counter();
        let mut choices = Choices::new(Color::Blue);
        choices.pause = true;
        assert!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            )
            .is_empty()
        );
        assert!(choices.pending);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.next_object_id_counter(), ids);
        choices.pause = false;
        choices.pending = false;
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Blue; 2]
        );
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 2);
    }
}
#[test]
fn cumulative_upkeep_secondary_bodies_pay_their_real_mana_and_life_costs() {
    for name in [
        "Infernal Darkness",
        "Naked Singularity",
        "Reality Twist",
        "Ritual of Subdual",
    ] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            game.turn.phase = ironsmith::game_state::Phase::Beginning;
            ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::Upkeep,
            )
            .advance(&mut game, &mut queue)
            .unwrap();
            for symbol in [
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
                ManaSymbol::Colorless,
            ] {
                game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
            }
            let mana = game.player(A).unwrap().mana_pool.total();
            let mut choices = Choices::new(Color::Blue);
            ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut game,
                &mut queue,
                &mut choices,
            )
            .unwrap();
            settle(&mut game, &mut choices);
            assert!(game.battlefield.contains(&source));
            assert_eq!(game.counter_count(source, ironsmith::CounterType::Age), 1);
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                mana - match name {
                    "Infernal Darkness" => 1,
                    "Ritual of Subdual" => 2,
                    _ => 3,
                }
            );
            assert_eq!(
                game.player(A).unwrap().life,
                if name == "Infernal Darkness" { 19 } else { 20 }
            );
        }
    }
}

fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending action without decision");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(!state.has_pending_action());
    settle(game, choices);
}
fn activate(game: &mut GameState, source: ObjectId, choices: &mut Choices) {
    game.remove_summoning_sickness(source);
    let ability_index = game
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
        .unwrap();
    announce(
        game,
        LegalAction::ActivateAbility {
            source,
            ability_index,
        },
        choices,
    );
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(
            |action| matches!(action, LegalAction::CastSpell {spell_id, ..} if *spell_id == source),
        )
        .unwrap();
    announce(game, action, choices);
    source
}
#[test]
fn harvest_pays_tap_discard_and_mana_then_registers_the_original_controllers_turn_scope() {
    for definition in definitions("Harvest Mage") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let discarded = card(&mut game, A, "Type: Sorcery\nDraw a card.", Zone::Hand);
        let source = land(&mut game, A, "Land");
        let foreign = land(&mut game, B, "Land");
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Green, 1);
        let mut choices = Choices::new(Color::Red);
        activate(&mut game, host, &mut choices);
        assert!(game.is_tapped(host));
        assert!(!game.player(A).unwrap().hand.contains(&discarded));
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Red]
        );
        assert_eq!(
            produce(
                &mut game,
                foreign,
                B,
                B,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Green; 2]
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Green; 2]
        );
    }
}
#[test]
fn quarum_announces_a_plains_then_its_indefinite_registration_rewrites_only_white_on_that_incarnation()
 {
    for definition in definitions("Quarum Trench Gnomes") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = land(&mut game, A, "Basic Land — Plains");
        let other = land(&mut game, A, "Land");
        let mut choices = Choices::new(Color::Red);
        activate(&mut game, host, &mut choices);
        assert!(game.is_tapped(host));
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        // The selection noun is not a repeating land-type condition.
        game.object_mut(source).unwrap().subtypes.clear();
        game.refresh_continuous_state().unwrap();
        let original = vec![ManaSymbol::White, ManaSymbol::Green, ManaSymbol::Colorless];
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                original.clone(),
                true,
                &mut choices
            ),
            vec![
                ManaSymbol::Colorless,
                ManaSymbol::Green,
                ManaSymbol::Colorless
            ]
        );
        assert_eq!(
            produce(&mut game, other, A, A, original.clone(), true, &mut choices),
            original
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::White; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Colorless; 2]
        );
        let departed = game.move_object_by_effect(source, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(departed, Zone::Battlefield)
            .unwrap();
        assert_ne!(returned, source);
        assert_eq!(
            produce(
                &mut game,
                returned,
                A,
                A,
                vec![ManaSymbol::White; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::White; 2]
        );
    }
}
#[test]
fn pale_moon_is_symmetric_nonbasic_only_preserves_amount_and_expires() {
    for definition in definitions("Pale Moon") {
        let mut game = game();
        let mut choices = Choices::new(Color::White);
        cast(&mut game, &definition, &mut choices);
        let own = land(&mut game, A, "Land");
        let foreign = land(&mut game, B, "Land");
        let basic = land(&mut game, A, "Basic Land — Forest");
        for (source, actor) in [(own, A), (foreign, B)] {
            assert_eq!(
                produce(
                    &mut game,
                    source,
                    actor,
                    actor,
                    vec![ManaSymbol::Green; 2],
                    true,
                    &mut choices
                ),
                vec![ManaSymbol::Colorless; 2]
            );
        }
        assert_eq!(
            produce(
                &mut game,
                basic,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Green; 2]
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            produce(
                &mut game,
                own,
                A,
                A,
                vec![ManaSymbol::Green; 2],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Green; 2]
        );
    }
}
#[test]
fn hall_upkeep_chooser_is_the_active_player_and_the_chosen_color_is_frozen_into_each_registration()
{
    for definition in definitions("Hall of Gemstone") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = land(&mut game, B, "Land");
        let mut choices = Choices::new(Color::Red);
        run_upkeep(&mut game, B, &mut choices);
        assert_eq!(choices.option_players, vec![B]);
        // An unrelated later change to source memory cannot retarget the
        // already-resolved turn-duration registration.
        game.set_chosen_color(host, Color::Blue);
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        assert_eq!(
            produce(
                &mut game,
                source,
                B,
                B,
                vec![ManaSymbol::Green, ManaSymbol::Colorless, ManaSymbol::White],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Red, ManaSymbol::Colorless, ManaSymbol::Red]
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            produce(
                &mut game,
                source,
                B,
                B,
                vec![ManaSymbol::Green],
                true,
                &mut choices
            ),
            vec![ManaSymbol::Green]
        );
    }
}
#[test]
fn false_dawn_rewrites_only_controlled_colored_production_draws_and_grants_white_only_spending_until_cleanup()
 {
    for definition in definitions("False Dawn") {
        let mut game = game();
        let drawn = card(&mut game, A, "Type: Land", Zone::Library);
        let drawn_stable = game.object(drawn).unwrap().stable_id;
        let mut choices = Choices::new(Color::White);
        cast(&mut game, &definition, &mut choices);
        let drawn = game.find_object_by_stable_id(drawn_stable).unwrap();
        assert!(game.player(A).unwrap().hand.contains(&drawn));
        game.player_mut(A).unwrap().mana_pool.empty();
        let source = card(&mut game, A, "Type: Artifact", Zone::Battlefield);
        assert_eq!(
            produce(
                &mut game,
                source,
                A,
                A,
                vec![ManaSymbol::Green, ManaSymbol::Colorless, ManaSymbol::Blue],
                false,
                &mut choices
            ),
            vec![ManaSymbol::White, ManaSymbol::Colorless, ManaSymbol::White]
        );
        assert_eq!(
            produce(
                &mut game,
                source,
                B,
                B,
                vec![ManaSymbol::Green],
                false,
                &mut choices
            ),
            vec![ManaSymbol::Green]
        );
        game.player_mut(A).unwrap().mana_pool.empty();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        let request = pay_request(&game, source, vec![ManaSymbol::Red]);
        let plan = plan_first_mana_payment(&game, &request).unwrap();
        execute_mana_payment_plan(&mut game, &request, &plan, &mut choices).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        assert!(
            check_mana_payment(&game, &pay_request(&game, source, vec![ManaSymbol::Blue])).is_ok()
        );
        assert!(
            check_mana_payment(&game, &pay_request(&game, source, vec![ManaSymbol::Red])).is_err(),
            "other mana keeps its normal color"
        );
        game.player_mut(A).unwrap().mana_pool.empty();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        assert!(
            check_mana_payment(
                &game,
                &pay_request(&game, source, vec![ManaSymbol::Colorless])
            )
            .is_err()
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        assert!(
            check_mana_payment(&game, &pay_request(&game, source, vec![ManaSymbol::Red])).is_err()
        );
    }
}

#[test]
fn contamination_upkeep_really_sacrifices_a_creature_or_its_own_source() {
    for definition in definitions("Contamination") {
        for offer_creature in [false, true] {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let creature = offer_creature.then(|| {
                card(
                    &mut game,
                    A,
                    "Type: Creature\nPower/Toughness: 2/2",
                    Zone::Battlefield,
                )
            });
            run_upkeep(&mut game, A, &mut Choices::new(Color::Blue));
            assert_eq!(game.battlefield.contains(&host), offer_creature);
            if let Some(creature) = creature {
                assert!(!game.battlefield.contains(&creature));
            }
            assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        }
    }
}
#[test]
fn departed_production_sources_match_their_recorded_characteristics() {
    for definition in definitions("Pulse of Llanowar") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = land(&mut game, A, "Basic Land — Forest");
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut choices = Choices::new(Color::Black);
        let mut ctx = EffectContext::new(source, A, &mut choices)
            .with_mana_production_provenance(ManaProductionProvenance::TappedSourceForMana);
        ctx.source_snapshot = Some(snapshot);
        let outcome = AddManaEffect::you(vec![ManaSymbol::Green; 2])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            outcome
                .events
                .iter()
                .find_map(|event| event.downcast::<ManaAddedEvent>())
                .unwrap()
                .mana,
            vec![ManaSymbol::Black; 2]
        );
        assert_eq!(game.player(A).unwrap().mana_pool.black, 2);
    }
}

#[test]
fn public_payment_query_preserves_an_unanswered_foreign_color_decision_as_typed_incomplete() {
    let mut game = game();
    let rule = ironsmith_core::ManaOutputRewrite {
        source_filter: ironsmith::target::ObjectFilter::land(),
        controller: None,
        tapped_for_mana: true,
        input: ironsmith_core::ManaRewriteInput::Any,
        output: ironsmith_core::ManaRewriteOutput::ChooseColor,
        quantity: ironsmith_core::ManaRewriteQuantity::Preserve,
    };
    let host = CardDefinitionBuilder::new(CardId::new(), "Foreign choice owner").card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(ironsmith::static_abilities::StaticAbility::mana_production_rewrite(rule,
            "If a land is tapped for mana, it produces mana of a color of your choice instead of any other type."))).build();
    game.create_object_from_definition(&host, B, Zone::Battlefield);
    let source = land(&mut game, A, "Land");
    let request = pay_request(&game, source, vec![ManaSymbol::Blue]);
    let result = check_mana_payment(&game, &request);
    assert!(
        matches!(
            result,
            Err(
                ironsmith::mana_payment::ManaPaymentFailure::EffectExecutionFailed(
                    ironsmith::effects::ExecutionError::UnresolvedPlayerDecision { player: B, .. }
                )
            )
        ),
        "{result:?}"
    );
    assert!(!game.is_tapped(source));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    let mut choices = Choices::new(Color::Blue);
    assert_eq!(
        produce(
            &mut game,
            source,
            A,
            A,
            vec![ManaSymbol::Green; 2],
            true,
            &mut choices
        ),
        vec![ManaSymbol::Blue; 2]
    );
    assert_eq!(
        choices.color_players,
        vec![B],
        "ordinary execution requests the actual owner's choice"
    );
}

//! Tezzeret the Seeker: "+1: Untap up to two target artifacts. −X: Search your
//! library for an artifact card with mana value X or less, put it onto the
//! battlefield, then shuffle. −5: Artifacts you control become artifact
//! creatures with base power and toughness 5/5 until end of turn."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Tezzeret the Seeker",
    )
    .unwrap()
    .remove(0)
}

#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(
        snapshot.parse_error.is_none() && !snapshot.parse_lossy && !snapshot.has_unimplemented,
        "{snapshot:#?}"
    );
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
}

#[derive(Default)]
struct Choices {
    x: u32,
    targets: Vec<ObjectId>,
    pick: Option<&'static str>,
}

impl DecisionMaker for Choices {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x } else { ctx.min }
    }

    fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
        self.targets.iter().map(|id| Target::Object(*id)).collect()
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|c| c.legal && Some(c.name.as_str()) == self.pick)
            .map(|c| c.id)
            .collect()
    }
}

fn artifact(name: &str, mana_value: u8) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build()
}

fn setup(loyalty: u32) -> (GameState, ObjectId) {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let tezzeret = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let printed = game
        .object(tezzeret)
        .unwrap()
        .counters
        .get(&CounterType::Loyalty)
        .copied()
        .unwrap_or(0);
    assert_eq!(printed, 4, "starting loyalty");
    if loyalty > printed {
        game.add_counters(tezzeret, CounterType::Loyalty, loyalty - printed);
    }
    (game, tezzeret)
}

/// Activates Tezzeret's `ordinal`-th loyalty ability and resolves it.
fn activate(game: &mut GameState, tezzeret: ObjectId, ordinal: usize, dm: &mut Choices) {
    let alice = PlayerId::from_index(0);
    let index = game
        .object(tezzeret)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(activated) if activated.is_loyalty_ability())
        })
        .nth(ordinal)
        .unwrap()
        .0;
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, ability_index, .. } if *source == tezzeret && *ability_index == index))
        .expect("loyalty ability offered");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..16 {
        if !game.stack.is_empty() || result.is_err() {
            break;
        }
        let Ok(GameProgress::NeedsDecisionCtx(ctx)) = result else {
            break;
        };
        result = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &ctx, dm,
        );
    }
    assert_eq!(game.stack.len(), 1, "{result:?}");
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

#[test]
fn plus_one_untaps_up_to_two_target_artifacts() {
    let (mut game, tezzeret) = setup(4);
    let alice = PlayerId::from_index(0);
    let a = game.create_object_from_definition(&artifact("Relic A", 1), alice, Zone::Battlefield);
    let b = game.create_object_from_definition(&artifact("Relic B", 1), alice, Zone::Battlefield);
    game.tap(a);
    game.tap(b);
    let mut dm = Choices {
        targets: vec![a, b],
        ..Default::default()
    };
    activate(&mut game, tezzeret, 0, &mut dm);
    assert!(!game.is_tapped(a) && !game.is_tapped(b));
    assert_eq!(
        game.object(tezzeret)
            .unwrap()
            .counters
            .get(&CounterType::Loyalty)
            .copied(),
        Some(5)
    );
}

#[test]
fn minus_x_puts_an_artifact_with_mana_value_x_or_less_onto_the_battlefield() {
    let (mut game, tezzeret) = setup(4);
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&artifact("Big Relic", 5), alice, Zone::Library);
    game.create_object_from_definition(&artifact("Small Relic", 2), alice, Zone::Library);
    let mut dm = Choices {
        x: 2,
        pick: Some("Small Relic"),
        ..Default::default()
    };
    activate(&mut game, tezzeret, 1, &mut dm);
    let on_battlefield = |name: &str| {
        game.battlefield
            .iter()
            .any(|id| game.object(*id).is_some_and(|o| o.name.as_str() == name))
    };
    assert!(on_battlefield("Small Relic"));
    assert!(!on_battlefield("Big Relic"), "mana value 5 is more than X");
    assert_eq!(
        game.object(tezzeret)
            .unwrap()
            .counters
            .get(&CounterType::Loyalty)
            .copied(),
        Some(2)
    );
}

#[test]
fn minus_five_turns_your_artifacts_into_five_five_creatures_until_end_of_turn() {
    let (mut game, tezzeret) = setup(6);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mine =
        game.create_object_from_definition(&artifact("My Relic", 1), alice, Zone::Battlefield);
    let theirs =
        game.create_object_from_definition(&artifact("Their Relic", 1), bob, Zone::Battlefield);
    let mut dm = Choices::default();
    activate(&mut game, tezzeret, 2, &mut dm);
    game.refresh_continuous_state();
    assert!(game.object_has_card_type(mine, CardType::Creature));
    assert!(game.object_has_card_type(mine, CardType::Artifact));
    assert_eq!(game.calculated_power(mine), Some(5));
    assert_eq!(game.calculated_toughness(mine), Some(5));
    assert!(
        !game.object_has_card_type(theirs, CardType::Creature),
        "only your artifacts"
    );
    ironsmith::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert!(
        !game.object_has_card_type(mine, CardType::Creature),
        "until end of turn"
    );
}

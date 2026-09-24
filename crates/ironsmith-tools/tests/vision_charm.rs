//! Vision Charm: "Choose one — • Target player mills four cards. • Choose a
//! land type and a basic land type. Each land of the first chosen type becomes
//! the second chosen type until end of turn. • Target artifact phases out."
//!
//! Modal cards render their modes from authored source text, so the
//! structural lowering of each mode is asserted here directly.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Subtype, Supertype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Vision Charm",
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

#[test]
fn land_type_mode_lowers_to_one_choice_of_each_type_applied_to_the_whole_set() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let debug = format!("{:#?}", def.spell_effect);
    assert!(debug.contains("ChooseLandTypeEffect"), "{debug}");
    assert!(debug.contains("BecomeBasicLandTypeChoiceEffect"), "{debug}");
    assert!(debug.contains("chosen_land_type: true"), "lands of the first chosen type");
    assert!(
        !debug.contains("ForEachObject"),
        "the basic land type is chosen once, not once per land"
    );
}

struct Choices {
    mode: usize,
    land_type: &'static str,
    basic_type: &'static str,
    target: Option<Target>,
    prompts: Vec<String>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.prompts.push(ctx.description.clone());
        if ctx.description.starts_with("Choose mode") {
            return vec![self.mode];
        }
        if !ctx.description.contains("land type") {
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .take(ctx.min.max(1))
                .map(|option| option.index)
                .collect();
        }
        let want = if ctx.description.contains("basic land type") {
            self.basic_type
        } else {
            self.land_type
        };
        ctx.options
            .iter()
            .filter(|option| option.description.eq_ignore_ascii_case(want))
            .map(|option| option.index)
            .take(1)
            .collect()
    }

    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let wanted = self.target.clone().expect("mode with a target");
        assert!(ctx.requirements[0].legal_targets.contains(&wanted));
        vec![wanted]
    }
}

fn land(name: &str, subtype: Subtype, basic: bool) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .subtypes(vec![subtype]);
    if basic {
        builder = builder.supertypes(vec![Supertype::Basic]);
    }
    builder.build()
}

fn cast(game: &mut GameState, dm: &mut Choices) {
    let alice = PlayerId::from_index(0);
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("instant is castable");
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..32 {
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
    assert_eq!(game.stack.len(), 1, "{result:?} prompts={:?}", dm.prompts);
    ironsmith::game_loop::resolve_stack_entry_with(game, dm).unwrap();
}

fn new_game() -> GameState {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game
}

#[test]
fn chosen_land_type_becomes_the_chosen_basic_type_until_end_of_turn() {
    let mut game = new_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alice_forest = game.create_object_from_definition(&land("Alice Forest", Subtype::Forest, true), alice, Zone::Battlefield);
    let bob_forest = game.create_object_from_definition(&land("Bob Forest", Subtype::Forest, true), bob, Zone::Battlefield);
    let mountain = game.create_object_from_definition(&land("Bob Mountain", Subtype::Mountain, true), bob, Zone::Battlefield);
    let mut dm = Choices {
        mode: 1,
        land_type: "Forest",
        basic_type: "Island",
        target: None,
        prompts: Vec::new(),
    };
    cast(&mut game, &mut dm);
    assert_eq!(
        dm.prompts.iter().filter(|p| p.contains("basic land type")).count(),
        1,
        "one basic land type choice for all lands: {:?}",
        dm.prompts
    );
    for forest in [alice_forest, bob_forest] {
        let subtypes = game.calculated_subtypes(forest);
        assert!(subtypes.contains(&Subtype::Island), "{subtypes:?}");
        assert!(!subtypes.contains(&Subtype::Forest), "setting a land type replaces it");
    }
    assert_eq!(game.calculated_subtypes(mountain), vec![Subtype::Mountain], "other lands unchanged");

    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(game.calculated_subtypes(alice_forest).contains(&Subtype::Forest), "until end of turn");
}

#[test]
fn mill_mode_mills_four_cards_from_the_target_player() {
    let mut game = new_game();
    let bob = PlayerId::from_index(1);
    for i in 0..6 {
        let card = CardDefinitionBuilder::new(CardId::new(), &format!("Card {i}"))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_definition(&card, bob, Zone::Library);
    }
    let mut dm = Choices {
        mode: 0,
        land_type: "",
        basic_type: "",
        target: Some(Target::Player(bob)),
        prompts: Vec::new(),
    };
    cast(&mut game, &mut dm);
    assert_eq!(game.player(bob).unwrap().library.len(), 2);
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 4);
}

#[test]
fn phase_out_mode_phases_out_the_target_artifact() {
    let mut game = new_game();
    let bob = PlayerId::from_index(1);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let relic: ObjectId = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
    let mut dm = Choices {
        mode: 2,
        land_type: "",
        basic_type: "",
        target: Some(Target::Object(relic)),
        prompts: Vec::new(),
    };
    cast(&mut game, &mut dm);
    assert!(game.is_phased_out(relic));
}

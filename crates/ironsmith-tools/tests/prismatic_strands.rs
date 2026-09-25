//! Prismatic Strands: "Prevent all damage that sources of the color of your
//! choice would deal this turn. Flashback—Tap an untapped white creature you
//! control."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::{Color, ColorSet};
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
use ironsmith::ids::CardId;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Prismatic Strands",
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

/// Picks the mode naming `color`, and taps `tapper` for flashback.
struct Choices {
    color: &'static str,
    tapper: Option<ObjectId>,
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let picked: Vec<usize> = ctx
            .options
            .iter()
            .filter(|o| o.legal && o.description.contains(&format!(" {} sources", self.color)))
            .map(|o| o.index)
            .collect();
        if picked.is_empty() {
            ctx.options
                .iter()
                .filter(|o| o.legal)
                .take(ctx.min.max(1))
                .map(|o| o.index)
                .collect()
        } else {
            picked
        }
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let tapper = self.tapper.expect("flashback cost");
        assert!(ctx.candidates.iter().any(|c| c.legal && c.id == tapper));
        vec![tapper]
    }
}

fn creature(name: &str, color: Color) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::from(color))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn setup() -> (GameState, ObjectId, ObjectId, ObjectId, ObjectId) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let red = game.create_object_from_definition(
        &creature("Red Source", Color::Red),
        bob,
        Zone::Battlefield,
    );
    let green = game.create_object_from_definition(
        &creature("Green Source", Color::Green),
        bob,
        Zone::Battlefield,
    );
    let white = game.create_object_from_definition(
        &creature("White Knight", Color::White),
        alice,
        Zone::Battlefield,
    );
    let target = game.create_object_from_definition(
        &creature("Alice Bear", Color::Green),
        alice,
        Zone::Battlefield,
    );
    (game, red, green, white, target)
}

fn cast(game: &mut GameState, spell: ObjectId, dm: &mut Choices) {
    let alice = PlayerId::from_index(0);
    let action = compute_legal_actions(game, alice)
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("castable");
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

fn hit(game: &mut GameState, source: ObjectId, target: ChooseSpec) {
    let bob = PlayerId::from_index(1);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, bob, &mut dm);
    ironsmith::effects::execute_effect(game, &ironsmith::Effect::deal_damage(2, target), &mut ctx)
        .unwrap();
}

#[test]
fn prevents_damage_from_sources_of_the_chosen_color_only() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let (mut game, red, green, _white, bear) = setup();
    let alice = PlayerId::from_index(0);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::White, 3);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    cast(
        &mut game,
        hand,
        &mut Choices {
            color: "red",
            tapper: None,
        },
    );

    hit(&mut game, red, ChooseSpec::SpecificObject(bear));
    hit(&mut game, red, ChooseSpec::Player(PlayerFilter::You));
    assert_eq!(
        game.damage_on(bear),
        0,
        "red damage to creatures is prevented"
    );
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "red damage to players is prevented"
    );

    hit(&mut game, green, ChooseSpec::SpecificObject(bear));
    assert_eq!(game.damage_on(bear), 2, "green damage is not prevented");

    ironsmith::turn::execute_cleanup_step(&mut game);
    game.turn.turn_number += 1;
    hit(&mut game, red, ChooseSpec::SpecificObject(bear));
    assert_eq!(game.damage_on(bear), 2, "the shield lasts only this turn");
}

#[test]
fn flashback_by_tapping_an_untapped_white_creature() {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let (mut game, _red, green, white, bear) = setup();
    let alice = PlayerId::from_index(0);
    let grave = game.create_object_from_definition(&def, alice, Zone::Graveyard);
    cast(
        &mut game,
        grave,
        &mut Choices {
            color: "green",
            tapper: Some(white),
        },
    );
    assert!(game.is_tapped(white), "tapped for flashback");
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "no mana needed"
    );
    hit(&mut game, green, ChooseSpec::SpecificObject(bear));
    assert_eq!(game.damage_on(bear), 0);
    assert!(
        game.exile.iter().any(|id| game
            .object(*id)
            .is_some_and(|o| o.name == "Prismatic Strands")),
        "exiled after flashback"
    );
}

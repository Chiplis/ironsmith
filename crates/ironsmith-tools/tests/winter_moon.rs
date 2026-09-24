//! Winter Moon: "Players can't untap more than one nonbasic land during their
//! untap steps."
use ironsmith::card::PowerToughness;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::events::cause::EventCause;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Supertype, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Winter Moon",
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

/// Picks the named candidate and records every prompt.
struct PickNamed {
    name: &'static str,
    prompts: Vec<(PlayerId, Vec<String>, usize, Option<usize>)>,
}

impl DecisionMaker for PickNamed {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.prompts.push((
            ctx.player,
            ctx.candidates.iter().map(|c| c.name.clone()).collect(),
            ctx.min,
            ctx.max,
        ));
        ctx.candidates
            .iter()
            .filter(|c| c.name == self.name)
            .map(|c| c.id)
            .collect()
    }
}

fn land(name: &str, basic: bool) -> ironsmith::cards::CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
    if basic {
        builder = builder.supertypes(vec![Supertype::Basic]);
    }
    builder.build()
}

struct Board {
    game: GameState,
    moon: ObjectId,
    nonbasic: Vec<ObjectId>,
    basics: Vec<ObjectId>,
    creature: ObjectId,
    untapped_nonbasic: ObjectId,
}

/// `player` controls three tapped nonbasic lands, one untapped nonbasic land,
/// two tapped basic lands and a tapped creature; Alice controls Winter Moon.
fn board(player: PlayerId) -> Board {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = player;
    let moon = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let nonbasic: Vec<_> = ["Nonbasic A", "Nonbasic B", "Nonbasic C"]
        .into_iter()
        .map(|name| game.create_object_from_definition(&land(name, false), player, Zone::Battlefield))
        .collect();
    let untapped_nonbasic =
        game.create_object_from_definition(&land("Untapped Nonbasic", false), player, Zone::Battlefield);
    let basics: Vec<_> = ["Basic A", "Basic B"]
        .into_iter()
        .map(|name| game.create_object_from_definition(&land(name, true), player, Zone::Battlefield))
        .collect();
    let creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        player,
        Zone::Battlefield,
    );
    for id in nonbasic.iter().chain(basics.iter()).chain([&creature]) {
        game.tap(*id);
    }
    Board {
        game,
        moon,
        nonbasic,
        basics,
        creature,
        untapped_nonbasic,
    }
}

fn untap_step(game: &mut GameState, dm: &mut PickNamed) {
    game.turn.phase = ironsmith::game_state::Phase::Beginning;
    game.turn.step = Some(ironsmith::game_state::Step::Untap);
    ironsmith::turn::execute_untap_step_with(game, dm);
}

#[test]
fn only_the_chosen_nonbasic_land_untaps_for_each_player() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let mut board = board(player);
        let mut dm = PickNamed {
            name: "Nonbasic B",
            prompts: Vec::new(),
        };
        untap_step(&mut board.game, &mut dm);
        let game = &board.game;
        assert_eq!(dm.prompts.len(), 1, "one limit prompt");
        let (chooser, candidates, min, max) = &dm.prompts[0];
        assert_eq!(*chooser, player, "the untapping player chooses");
        assert_eq!((*min, *max), (1, Some(1)));
        let mut candidates = candidates.clone();
        candidates.sort();
        assert_eq!(candidates, vec!["Nonbasic A", "Nonbasic B", "Nonbasic C"], "only tapped nonbasic lands are limited");
        assert!(!game.is_tapped(board.nonbasic[1]), "the chosen nonbasic land untaps");
        assert!(game.is_tapped(board.nonbasic[0]) && game.is_tapped(board.nonbasic[2]));
        assert!(board.basics.iter().all(|id| !game.is_tapped(*id)), "basic lands untap normally");
        assert!(!game.is_tapped(board.creature), "other permanents untap normally");
        assert!(!game.is_tapped(board.untapped_nonbasic));
    }
}

#[test]
fn no_choice_when_at_most_one_nonbasic_land_is_tapped() {
    let mut board = board(PlayerId::from_index(1));
    board.game.untap(board.nonbasic[0]);
    board.game.untap(board.nonbasic[1]);
    let mut dm = PickNamed {
        name: "none",
        prompts: Vec::new(),
    };
    untap_step(&mut board.game, &mut dm);
    assert!(dm.prompts.is_empty());
    assert!(board.nonbasic.iter().all(|id| !board.game.is_tapped(*id)));
}

#[test]
fn limit_ends_when_winter_moon_leaves() {
    let mut board = board(PlayerId::from_index(1));
    board
        .game
        .move_object(board.moon, Zone::Graveyard, EventCause::effect());
    let mut dm = PickNamed {
        name: "none",
        prompts: Vec::new(),
    };
    untap_step(&mut board.game, &mut dm);
    assert!(dm.prompts.is_empty());
    assert!(board.nonbasic.iter().all(|id| !board.game.is_tapped(*id)));
}

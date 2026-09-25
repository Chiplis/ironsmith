//! Vendilion Clique: "Flash. Flying. When Vendilion Clique enters, look at
//! target player's hand. You may choose a nonland card from it. If you do, that
//! player reveals the chosen card, puts it on the bottom of their library, then
//! draws a card."
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_state::Target;
use ironsmith::ids::CardId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Vendilion Clique",
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

struct Choices {
    target: PlayerId,
    choose: bool,
    offered: Vec<String>,
}

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(
            ctx.requirements[0]
                .legal_targets
                .contains(&Target::Player(self.target))
        );
        vec![Target::Player(self.target)]
    }

    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        self.choose
    }

    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.offered = ctx
            .candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.name.to_string())
            .collect();
        if !self.choose {
            return Vec::new();
        }
        ctx.candidates
            .iter()
            .filter(|c| c.legal && c.name.as_str() == "Lightning Bolt")
            .map(|c| c.id)
            .collect()
    }
}

fn card(name: &str, card_type: CardType) -> ironsmith::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn names(game: &GameState, ids: impl Iterator<Item = ObjectId>) -> Vec<String> {
    ids.map(|id| game.object(id).unwrap().name.to_string())
        .collect()
}

/// Bob holds Forest, Lightning Bolt, and Counterspell with Top Card on his library over
/// Bottom Card. Vendilion Clique enters under Alice's control targeting Bob.
fn run(choose: bool) -> (GameState, Choices) {
    let def = ironsmith_tools::compile_definition_from_payload(&payload()).unwrap();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.create_object_from_definition(&card("Bottom Card", CardType::Sorcery), bob, Zone::Library);
    game.create_object_from_definition(&card("Top Card", CardType::Sorcery), bob, Zone::Library);
    game.create_object_from_definition(&card("Forest", CardType::Land), bob, Zone::Hand);
    game.create_object_from_definition(&card("Lightning Bolt", CardType::Instant), bob, Zone::Hand);
    game.create_object_from_definition(&card("Counterspell", CardType::Instant), bob, Zone::Hand);
    let hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let mut dm = Choices {
        target: bob,
        choose,
        offered: Vec::new(),
    };
    game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, &mut dm)
        .expect("enters");
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        for entry in ironsmith::triggers::check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1, "the enter trigger");
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    (game, dm)
}

#[test]
fn chosen_nonland_card_goes_to_the_bottom_and_its_owner_draws() {
    let (game, dm) = run(true);
    let bob = PlayerId::from_index(1);
    let player = game.player(bob).unwrap();
    let mut hand = names(&game, player.hand.iter().copied());
    hand.sort();
    let mut offered = dm.offered.clone();
    offered.sort();
    assert_eq!(
        offered,
        vec!["Counterspell".to_string(), "Lightning Bolt".to_string()],
        "lands can't be chosen"
    );
    assert_eq!(
        hand,
        vec![
            "Counterspell".to_string(),
            "Forest".to_string(),
            "Top Card".to_string()
        ],
        "drew a replacement card"
    );
    let library = names(&game, player.library.iter().copied());
    assert_eq!(
        library.first().map(String::as_str),
        Some("Lightning Bolt"),
        "on the bottom: {library:?}"
    );
    assert_eq!(library.len(), 2);
}

#[test]
fn choosing_nothing_leaves_the_hand_and_draws_nothing() {
    let (game, _) = run(false);
    let bob = PlayerId::from_index(1);
    let player = game.player(bob).unwrap();
    let mut hand = names(&game, player.hand.iter().copied());
    hand.sort();
    assert_eq!(
        hand,
        vec![
            "Counterspell".to_string(),
            "Forest".to_string(),
            "Lightning Bolt".to_string()
        ]
    );
    assert_eq!(player.library.len(), 2);
}

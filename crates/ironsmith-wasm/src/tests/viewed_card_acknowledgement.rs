use crate::{GameSnapshot, ReplayDecisionAnswer, WasmReplayDecisionMaker};
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{
    OrderContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, SelectableObject,
    SelectableOption, ViewCardsContext,
};
use ironsmith::game_state::GameState;
use ironsmith::ids::{ObjectId, PlayerId};
use ironsmith::zone::Zone;
use ironsmith_registry_test::cards::definitions::ornithopter;

fn fixture() -> (GameState, PlayerId, PlayerId, ObjectId, Vec<ObjectId>) {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);
    let cards = (0..3)
        .map(|_| game.create_object_from_definition(&ornithopter(), bob, Zone::Hand))
        .collect();
    (game, alice, bob, source, cards)
}

fn view(
    game: &GameState,
    dm: &mut WasmReplayDecisionMaker,
    viewer: PlayerId,
    subject: PlayerId,
    source: ObjectId,
    cards: &[ObjectId],
    public: bool,
) {
    let ctx = ViewCardsContext::new(viewer, subject, Some(source), Zone::Hand, "Inspect cards")
        .with_public(public);
    dm.view_cards(game, viewer, cards, &ctx);
}

fn selection(player: PlayerId, source: ObjectId, cards: &[ObjectId]) -> SelectObjectsContext {
    SelectObjectsContext::new(
        player,
        Some(source),
        "Pick a card",
        cards
            .iter()
            .map(|id| SelectableObject::new(*id, "Ornithopter"))
            .collect(),
        0,
        Some(1),
    )
}

#[test]
fn viewed_card_ack_requires_a_submitted_prompt_covering_the_entire_view() {
    let (game, alice, bob, source, cards) = fixture();
    let mut pending = WasmReplayDecisionMaker::new(&[]);
    view(&game, &mut pending, alice, bob, source, &cards, false);
    pending.decide_objects(&game, &selection(alice, source, &cards));
    assert!(pending.viewed_cards.unwrap().acknowledged_by.is_empty());

    for (player, prompt_source, prompt_cards, expected) in [
        (alice, source, cards.clone(), true),
        (bob, source, cards.clone(), false),
        (alice, cards[0], cards.clone(), false),
        (alice, source, cards[..1].to_vec(), false),
    ] {
        let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(Vec::new())]);
        view(&game, &mut dm, alice, bob, source, &cards, false);
        dm.decide_objects(&game, &selection(player, prompt_source, &prompt_cards));
        let (_, viewed, audit, _) = dm.finish();
        assert_eq!(viewed.unwrap().acknowledged_by.contains(&alice), expected);
        assert_eq!(
            audit.len(),
            1,
            "acknowledgement preserves the visibility audit"
        );
    }
}

#[test]
fn viewed_card_ack_partition_order_and_object_options_are_structural() {
    let (game, alice, bob, source, cards) = fixture();
    let items: Vec<_> = cards
        .iter()
        .map(|id| (*id, "Ornithopter".to_string()))
        .collect();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Partition(Vec::new())]);
    view(&game, &mut dm, alice, bob, source, &cards, false);
    dm.decide_partition(
        &game,
        &PartitionContext::new(
            alice,
            Some(source),
            "Divide these cards",
            items.clone(),
            "pile A",
            "pile B",
        ),
    );
    assert_eq!(dm.viewed_cards.unwrap().acknowledged_by, vec![alice]);

    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Order(cards.clone())]);
    view(&game, &mut dm, alice, bob, source, &cards, false);
    dm.decide_order(
        &game,
        &OrderContext::new(alice, Some(source), "Arrange these cards", items),
    );
    assert_eq!(dm.viewed_cards.unwrap().acknowledged_by, vec![alice]);

    let options = cards
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let mut option = SelectableOption::new(index, "Card choice");
            option.object_id = Some(*id);
            option
        })
        .collect();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Options(vec![0])]);
    view(&game, &mut dm, alice, bob, source, &cards, false);
    dm.decide_options(
        &game,
        &SelectOptionsContext::new(alice, Some(source), "Pick one", options, 1, 1),
    );
    assert_eq!(dm.viewed_cards.unwrap().acknowledged_by, vec![alice]);
}

#[test]
fn viewed_card_ack_is_per_player_and_does_not_hide_public_cards() {
    let (game, alice, bob, source, cards) = fixture();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(vec![cards[0]])]);
    view(&game, &mut dm, alice, bob, source, &cards, true);
    dm.decide_objects(&game, &selection(alice, source, &cards));
    let viewed = dm.viewed_cards.unwrap();
    for (perspective, expected) in [(alice, true), (bob, false)] {
        let snapshot = GameSnapshot::from_game(
            &game,
            perspective,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            Some(&viewed),
            false,
            None,
            0,
        );
        let snapshot_view = snapshot.viewed_cards.unwrap();
        assert_eq!(snapshot_view.acknowledged, expected);
        assert_eq!(snapshot_view.cards.len(), 3);
    }
}

#[test]
fn viewed_card_ack_a_new_identical_view_needs_acknowledgement_again() {
    let (game, alice, bob, source, cards) = fixture();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(Vec::new())]);
    view(&game, &mut dm, alice, bob, source, &cards, false);
    dm.decide_objects(&game, &selection(alice, source, &cards));
    assert_eq!(
        dm.viewed_cards.as_ref().unwrap().acknowledged_by,
        vec![alice]
    );
    view(&game, &mut dm, alice, bob, source, &cards, false);
    assert!(dm.viewed_cards.unwrap().acknowledged_by.is_empty());
}

#[test]
fn viewed_card_ack_follows_stable_card_identity_after_a_zone_change() {
    let (mut game, alice, bob, source, cards) = fixture();
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(Vec::new())]);
    view(&game, &mut dm, alice, bob, source, &cards, false);
    let moved = game.move_object_by_effect(cards[0], Zone::Exile).unwrap();
    let mut current_cards = cards.clone();
    current_cards[0] = moved;
    dm.decide_objects(&game, &selection(alice, source, &current_cards));
    assert_eq!(dm.viewed_cards.unwrap().acknowledged_by, vec![alice]);
}

#[test]
fn viewed_card_ack_real_library_arrangements_acknowledge_the_replayed_view() {
    use ironsmith::effects::{EffectContext, EffectExecutor, ScryEffect, SurveilEffect};
    for effect in [
        Box::new(ScryEffect::you(3)) as Box<dyn EffectExecutor>,
        Box::new(SurveilEffect::you(3)),
    ] {
        let (mut game, alice, _, source, _) = fixture();
        let cards: Vec<_> = (0..3)
            .map(|_| game.create_object_from_definition(&ornithopter(), alice, Zone::Library))
            .collect();
        let checkpoint = game.clone();
        let mut pending = WasmReplayDecisionMaker::new(&[]);
        effect
            .execute(
                &mut game,
                &mut EffectContext::new(source, alice, &mut pending),
            )
            .unwrap();
        assert!(pending.pending_context.is_some());
        assert!(pending.viewed_cards.unwrap().acknowledged_by.is_empty());

        game = checkpoint;
        let mut dm = WasmReplayDecisionMaker::new(&[
            ReplayDecisionAnswer::Partition(Vec::new()),
            ReplayDecisionAnswer::Order(cards.clone()),
        ]);
        effect
            .execute(&mut game, &mut EffectContext::new(source, alice, &mut dm))
            .unwrap();
        assert!(dm.pending_context.is_none());
        assert_eq!(dm.viewed_cards.unwrap().acknowledged_by, vec![alice]);
    }
}

#[test]
fn viewed_card_ack_a_card_prompt_can_supply_its_own_hidden_view() {
    use ironsmith::decisions::context::DecisionHiddenCardVisibility;
    let (game, alice, _, source, cards) = fixture();
    let ctx = selection(alice, source, &cards).with_hidden_card_view(
        cards.clone(),
        DecisionHiddenCardVisibility::PrivateToDecisionPlayer,
        "Choose from this hand",
    );
    let mut dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(vec![cards[0]])]);
    dm.decide_objects(&game, &ctx);
    let view = dm.viewed_cards.unwrap();
    assert_eq!(view.cards, cards);
    assert_eq!(view.acknowledged_by, vec![alice]);
}

#[test]
fn viewed_card_ack_public_carry_does_not_acknowledge_cards_outside_the_prompt() {
    let (game, alice, bob, source, cards) = fixture();
    let mut carry_dm = WasmReplayDecisionMaker::new(&[]);
    view(&game, &mut carry_dm, alice, bob, source, &cards, true);
    let mut next_dm = WasmReplayDecisionMaker::new(&[ReplayDecisionAnswer::Objects(Vec::new())]);
    view(&game, &mut next_dm, alice, bob, source, &cards[..1], true);
    next_dm.decide_objects(&game, &selection(alice, source, &cards[..1]));
    let merged =
        crate::merge_carried_active_viewed_cards(carry_dm.viewed_cards, next_dm.viewed_cards)
            .unwrap();
    assert_eq!(merged.cards.len(), 3);
    assert!(merged.acknowledged_by.is_empty());
}

#[test]
fn viewed_card_ack_stack_source_visibility_is_inspection_not_a_second_reveal() {
    use ironsmith::game_state::StackEntry;
    use ironsmith::snapshot::ObjectSnapshot;
    let (mut game, alice, bob, source, cards) = fixture();
    let snapshot = ObjectSnapshot::from_object(game.object(cards[0]).unwrap(), &game);
    game.stack.push(
        StackEntry::ability(source, alice, Vec::<ironsmith::effect::Effect>::new())
            .with_source_snapshot(snapshot),
    );
    let mut dm = WasmReplayDecisionMaker::new(&[]);
    view(&game, &mut dm, alice, bob, source, &cards, true);
    for perspective in [alice, bob] {
        let explicit = GameSnapshot::from_game(
            &game,
            perspective,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            dm.viewed_cards.as_ref(),
            false,
            None,
            0,
        );
        assert!(
            !explicit.viewed_cards.unwrap().inspector_only,
            "an actual reveal still needs acknowledgement"
        );
        let stack = GameSnapshot::from_game(
            &game,
            perspective,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            1,
        );
        let inspected = stack.viewed_cards.unwrap();
        assert!(inspected.inspector_only);
        assert_eq!(inspected.cards[0].name, "Ornithopter");
        assert_eq!(inspected.visibility, "public");
        assert!(
            stack.players[bob.index()]
                .hand_cards
                .iter()
                .any(|card| card.id == cards[0].0 && card.name == "Ornithopter")
        );
    }
}

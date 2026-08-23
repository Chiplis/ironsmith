use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::{BooleanContext, SelectOptionsContext};
use ironsmith::effects::execute_effect;
use ironsmith::events::other::DieRolledEvent;
use ironsmith::{
    Ability, CardBuilder, CardDefinition, CardId, CardType, Effect, EffectContext, GameState,
    ManaCost, ManaSymbol, PlayerFilter, PlayerId, StaticAbility, Zone,
};

struct AcceptFirst {
    expected_player: PlayerId,
}

impl DecisionMaker for AcceptFirst {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        assert_eq!(ctx.player, self.expected_player);
        true
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        assert_eq!(ctx.player, self.expected_player);
        vec![0]
    }
}

struct ChooseSecond;

impl DecisionMaker for ChooseSecond {
    fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
        vec![1]
    }
}

fn game() -> (GameState, PlayerId, PlayerId) {
    (
        GameState::new(vec!["Alice".into(), "Bob".into()], 20),
        PlayerId::from_index(0),
        PlayerId::from_index(1),
    )
}

fn modifier_source(
    game: &mut GameState,
    controller: PlayerId,
    abilities: Vec<StaticAbility>,
) -> ironsmith::ObjectId {
    let mut definition = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Die Modifier")
            .card_types(vec![CardType::Enchantment])
            .build(),
    );
    definition
        .abilities
        .extend(abilities.into_iter().map(Ability::static_ability));
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn generic_one() -> ManaCost {
    ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])
}

#[test]
fn u045_paid_reroll_replaces_the_natural_result_without_completing_the_discarded_roll() {
    let (mut game, alice, _) = game();
    let source = modifier_source(
        &mut game,
        alice,
        vec![StaticAbility::die_roll_reroll(
            PlayerFilter::You,
            generic_one(),
            true,
            "Once each turn, you may pay {1} to reroll one or more dice you rolled.",
        )],
    );
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    game.force_next_die_roll(2);
    game.force_next_die_roll(5);
    let mut decisions = AcceptFirst {
        expected_player: alice,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");

    assert_eq!(outcome.as_count(), Some(5));
    assert_eq!(outcome.events.len(), 1);
    let event = outcome.events[0]
        .downcast::<DieRolledEvent>()
        .expect("one completed die roll");
    assert_eq!((event.natural_result, event.result), (5, 5));
    assert!(
        !game
            .turn_store
            .turn_history
            .player_rolled_result_this_turn(alice, 2)
    );
    assert!(
        game.turn_store
            .turn_history
            .player_rolled_result_this_turn(alice, 5)
    );
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
}

#[test]
fn u045_rerolls_apply_before_numerical_modifiers() {
    let (mut game, alice, _) = game();
    let source = modifier_source(
        &mut game,
        alice,
        vec![
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                1,
                1,
                true,
                "Increase or decrease a die result.",
            ),
            StaticAbility::die_roll_reroll(PlayerFilter::You, generic_one(), true, "Reroll a die."),
        ],
    );
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);
    game.force_next_die_roll(2);
    game.force_next_die_roll(4);
    let mut decisions = AcceptFirst {
        expected_player: alice,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");
    let event = outcome.events[0]
        .downcast::<DieRolledEvent>()
        .expect("completed roll");

    assert_eq!((event.natural_result, event.result), (4, 5));
    assert_eq!(game.player(alice).expect("Alice").life, 19);
}

#[test]
fn i048_roller_makes_an_opponents_modifier_choices() {
    let (mut game, alice, bob) = game();
    let source = modifier_source(
        &mut game,
        bob,
        vec![StaticAbility::die_roll_result_adjustment(
            PlayerFilter::Opponent,
            1,
            1,
            true,
            "An opponent may adjust their die.",
        )],
    );
    game.force_next_die_roll(3);
    let mut decisions = AcceptFirst {
        expected_player: alice,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");

    assert_eq!(outcome.as_count(), Some(4));
    assert_eq!(game.player(alice).expect("Alice").life, 19);
    assert_eq!(game.player(bob).expect("Bob").life, 20);
}

#[test]
fn i048_multiple_modifier_instances_on_one_permanent_are_each_available() {
    let (mut game, alice, _) = game();
    let source = modifier_source(
        &mut game,
        alice,
        vec![
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                1,
                1,
                true,
                "First adjustment.",
            ),
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                1,
                1,
                true,
                "Second adjustment.",
            ),
        ],
    );
    game.force_next_die_roll(2);
    let mut decisions = AcceptFirst {
        expected_player: alice,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");

    assert_eq!(outcome.as_count(), Some(4));
    assert_eq!(game.player(alice).expect("Alice").life, 18);
    assert_eq!(
        game.turn_store
            .turn_history
            .die_roll_result_adjustments_this_turn
            .len(),
        2
    );
}

#[test]
fn u045_multiple_optional_rerolls_are_ordered_and_applied_independently() {
    let (mut game, alice, _) = game();
    let source = modifier_source(
        &mut game,
        alice,
        vec![
            StaticAbility::die_roll_reroll(PlayerFilter::You, generic_one(), true, "Reroll A."),
            StaticAbility::die_roll_reroll(PlayerFilter::You, generic_one(), true, "Reroll B."),
        ],
    );
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    game.force_next_die_roll(2);
    game.force_next_die_roll(3);
    game.force_next_die_roll(5);
    let mut decisions = AcceptFirst {
        expected_player: alice,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(&mut game, &Effect::roll_die(6, PlayerFilter::You), &mut ctx)
        .expect("roll resolves");

    assert_eq!(outcome.as_count(), Some(5));
    assert_eq!(outcome.events.len(), 1);
    assert_eq!(
        game.turn_store
            .turn_history
            .die_roll_result_adjustments_this_turn
            .len(),
        2
    );
}

#[test]
fn i049_unchosen_group_rolls_emit_no_event_ui_or_history_entry() {
    let (mut game, alice, _) = game();
    let source = game.new_object_id();
    game.force_next_die_roll(6);
    game.force_next_die_roll(2);
    let mut decisions = ChooseSecond;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(
        &mut game,
        &Effect::roll_dice_choose_result_with_die_text(
            2,
            6,
            PlayerFilter::You,
            Some("d6".to_string()),
        ),
        &mut ctx,
    )
    .expect("roll and choose resolves");

    assert_eq!(outcome.as_count(), Some(2));
    assert_eq!(outcome.events.len(), 1);
    assert!(
        !game
            .turn_store
            .turn_history
            .player_rolled_result_this_turn(alice, 6)
    );
    assert!(
        game.turn_store
            .turn_history
            .player_rolled_result_this_turn(alice, 2)
    );
}

#[test]
fn i049_tied_group_results_still_complete_exactly_one_roll() {
    let (mut game, alice, _) = game();
    let source = game.new_object_id();
    game.force_next_die_roll(4);
    game.force_next_die_roll(4);
    let mut decisions = ChooseSecond;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);

    let outcome = execute_effect(
        &mut game,
        &Effect::roll_dice_choose_result_with_die_text(
            2,
            6,
            PlayerFilter::You,
            Some("d6".to_string()),
        ),
        &mut ctx,
    )
    .expect("roll and choose resolves");

    assert_eq!(outcome.events.len(), 1);
    assert_eq!(
        game.turn_store.turn_history.die_rolls_this_turn[&alice],
        vec![4]
    );
}

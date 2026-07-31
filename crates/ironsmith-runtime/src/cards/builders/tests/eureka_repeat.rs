#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::effect::EffectPredicate;
use crate::effects::{
    ExecutionContext, ForPlayersEffect, MayEffect, RepeatProcessEffect, WithIdEffect,
};

const EUREKA_ORACLE: &str = "Starting with you, each player may put a permanent card from their hand onto the battlefield. Repeat this process until no one puts a card onto the battlefield.";

fn repeat_process(definition: &CardDefinition) -> &RepeatProcessEffect {
    definition
        .spell_effect
        .as_ref()
        .expect("Eureka should have a spell effect")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<RepeatProcessEffect>())
        .expect("the two authored sentences should lower to one repeat process")
}

#[test]
fn eureka_keeps_one_ordered_each_player_optional_repeat_process() {
    let definition = parse_oracle_card_definition("Eureka");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        EUREKA_ORACLE
    );

    let repeat = repeat_process(&definition);
    assert_eq!(repeat.predicate, EffectPredicate::Happened);
    let [condition] = repeat.effects.as_slice() else {
        panic!("the repeat body should be one each-player action: {repeat:#?}");
    };
    let condition = condition
        .downcast_ref::<WithIdEffect>()
        .expect("the each-player result should drive the loop");
    assert_eq!(condition.id, repeat.condition);
    let for_players = condition
        .effect
        .downcast_ref::<ForPlayersEffect>()
        .expect("the loop body should remain a typed player iteration");
    assert_eq!(for_players.filter, PlayerFilter::Any);
    assert!(for_players.starting_with_controller);
    assert!(!for_players.stop_after_first_happened);
    let [may] = for_players.effects.as_slice() else {
        panic!("each participant should receive one optional action");
    };
    let may = may
        .downcast_ref::<MayEffect>()
        .expect("the participant action should remain optional");
    assert_eq!(may.decider, Some(PlayerFilter::IteratedPlayer));
    assert_eq!(may.effects.len(), 2, "{may:#?}");
    assert!(
        may.effects[0]
            .downcast_ref::<ChooseObjectsEffect>()
            .is_some(),
        "the optional action should choose a permanent card from that player's hand"
    );
    assert!(
        may.effects[1]
            .downcast_ref::<TaggedEffect>()
            .and_then(|tagged| tagged.effect.downcast_ref::<MoveToZoneEffect>())
            .is_some_and(|move_to_zone| move_to_zone.zone == Zone::Battlefield),
        "the chosen card should move onto the battlefield"
    );
}

#[derive(Debug)]
struct EurekaDecisions {
    answers: Vec<bool>,
    next_answer: usize,
    prompted_players: Vec<PlayerId>,
}

impl DecisionMaker for EurekaDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.prompted_players.push(ctx.player);
        let answer = self
            .answers
            .get(self.next_answer)
            .copied()
            .expect("the process asked for more optional-action decisions than expected");
        self.next_answer += 1;
        answer
    }
}

fn permanent(raw_id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

#[test]
fn eureka_starts_with_controller_and_repeats_until_everyone_declines() {
    let definition = parse_oracle_card_definition("Eureka");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Eureka should have a spell effect");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&definition, bob, Zone::Stack);
    let alice_card = game.create_object_from_definition(
        &permanent(99_201, "Alice Permanent"),
        alice,
        Zone::Hand,
    );
    let bob_card =
        game.create_object_from_definition(&permanent(99_202, "Bob Permanent"), bob, Zone::Hand);
    let bob_spare = game.create_object_from_definition(
        &permanent(99_203, "Bob Spare Permanent"),
        bob,
        Zone::Hand,
    );
    let stable_id = |game: &crate::game_state::GameState, object| {
        game.object(object)
            .expect("the created object should exist")
            .stable_id
    };
    let alice_stable = stable_id(&game, alice_card);
    let bob_stable = stable_id(&game, bob_card);
    let bob_spare_stable = stable_id(&game, bob_spare);

    let mut decisions = EurekaDecisions {
        // Bob accepts in round one; Alice declines. Both decline in round two.
        answers: vec![true, false, false, false],
        next_answer: 0,
        prompted_players: Vec::new(),
    };
    let mut ctx = ExecutionContext::new(source, bob, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        bob,
        source,
        program,
        None,
        &[],
    )
    .expect("the ordered optional repeat process should resolve");
    drop(ctx);

    assert_eq!(
        decisions.prompted_players,
        vec![bob, alice, bob, alice],
        "each round should begin with the resolving spell's controller"
    );
    assert_eq!(decisions.next_answer, 4);
    let current_zone = |stable| {
        let current = game
            .find_object_by_stable_id(stable)
            .expect("the card should retain stable identity");
        game.object(current)
            .expect("the current card object should exist")
            .zone
    };
    assert_eq!(current_zone(bob_stable), Zone::Battlefield);
    assert_eq!(
        current_zone(bob_spare_stable),
        Zone::Hand,
        "declining in the second round should leave the spare card in hand"
    );
    assert_eq!(current_zone(alice_stable), Zone::Hand);
}

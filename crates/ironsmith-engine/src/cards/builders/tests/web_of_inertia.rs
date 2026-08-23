#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;

struct MayDecision {
    accept: bool,
    expected_player: PlayerId,
    calls: usize,
}

impl DecisionMaker for MayDecision {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert_eq!(
            ctx.player, self.expected_player,
            "the opponent whose combat is beginning must make the choice"
        );
        self.calls += 1;
        self.accept
    }
}

fn resolve_combat_trigger(
    accept: bool,
) -> (
    crate::game_state::GameState,
    ObjectId,
    crate::ids::StableId,
    MayDecision,
) {
    let definition = parse_oracle_card_definition("Web of Inertia");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Web of Inertia should have a beginning-of-combat trigger");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let web = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let attacker = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Bob's Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(attacker);
    let graveyard_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Exile Fodder")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        bob,
        Zone::Graveyard,
    );
    let graveyard_card_stable_id = game
        .object(graveyard_card)
        .expect("graveyard card should exist")
        .stable_id;

    let mut decisions = MayDecision {
        accept,
        expected_player: bob,
        calls: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(web, alice, &mut decisions);
    ctx.iteration.iterated_player = Some(bob);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        web,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Web of Inertia's combat trigger should resolve");
    drop(ctx);

    (game, attacker, graveyard_card_stable_id, decisions)
}

#[test]
fn web_of_inertia_binds_the_opponents_choice_and_attack_restriction() {
    let (declined, declined_attacker, declined_card, declined_decisions) =
        resolve_combat_trigger(false);
    let alice = PlayerId::from_index(0);

    assert_eq!(declined_decisions.calls, 1);
    assert_eq!(
        declined
            .find_object_by_stable_id(declined_card)
            .and_then(|id| declined.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard),
        "declining must leave the opponent's graveyard card in place"
    );
    assert!(
        !declined.can_attack_player_directly(declined_attacker, alice),
        "that opponent's creatures must not be able to attack Web's controller directly"
    );
    assert!(
        declined.can_attack_defending_player(declined_attacker, alice),
        "the authored player-only restriction must not also ban attacks on that player's planeswalkers"
    );

    let (accepted, accepted_attacker, accepted_card, accepted_decisions) =
        resolve_combat_trigger(true);
    assert_eq!(accepted_decisions.calls, 1);
    assert_eq!(
        accepted
            .find_object_by_stable_id(accepted_card)
            .and_then(|id| accepted.object(id))
            .map(|object| object.zone),
        Some(Zone::Exile),
        "accepting must exile a card from that opponent's graveyard"
    );
    assert!(
        accepted.can_attack_player_directly(accepted_attacker, alice),
        "successfully exiling the card must suppress the DidNot consequence"
    );

    let rendered = canonical_compiled_lines(&parse_oracle_card_definition("Web of Inertia"))
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("at the beginning of combat on each opponent's turn")
            && rendered.contains("can't attack you this turn")
            && !rendered.contains("can't attack you or planeswalkers you control this turn"),
        "the compiled surface must retain the trigger and player-only restriction: {rendered}"
    );
}

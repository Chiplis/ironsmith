#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

struct LukkaMayDecisionMaker {
    accept: bool,
}

impl crate::decision::DecisionMaker for LukkaMayDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }
}

fn lukka_plus_one_program(
    definition: &crate::cards::CardDefinition,
) -> crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.is_loyalty_ability
                    && format!("{:?}", activated.effects).contains("DiscardEffect") =>
            {
                Some(activated.effects.clone())
            }
            _ => None,
        })
        .expect("Lukka should have a +1 loyalty ability that may discard a card")
}

fn resolve_lukka_plus_one(
    discard_type: CardType,
    accept: bool,
) -> (crate::game_state::GameState, PlayerId) {
    let definition = parse_oracle_card_definition("Lukka, Wayward Bonder");
    let program = lukka_plus_one_program(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let lukka = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let discard = CardDefinitionBuilder::new(CardId::new(), "Discard Candidate")
        .card_types(vec![discard_type])
        .build();
    game.create_object_from_definition(&discard, alice, Zone::Hand);
    let draw = CardDefinitionBuilder::new(CardId::new(), "Draw Card")
        .card_types(vec![CardType::Artifact])
        .build();
    for _ in 0..4 {
        game.create_object_from_definition(&draw, alice, Zone::Library);
    }

    let mut decisions = LukkaMayDecisionMaker { accept };
    let mut ctx = crate::effects::ExecutionContext::new(lukka, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        lukka,
        &program,
        None,
        &[],
    )
    .expect("Lukka's +1 loyalty effect should resolve");
    (game, alice)
}

#[test]
fn lukka_plus_one_decline_discards_and_draws_nothing() {
    let (game, alice) = resolve_lukka_plus_one(CardType::Creature, false);
    let player = game.player(alice).expect("Alice should exist");

    assert_eq!(
        player.hand.len(),
        1,
        "declining must leave the card in hand"
    );
    assert_eq!(player.library.len(), 4, "declining must draw no cards");
    assert!(
        player.graveyard.is_empty(),
        "declining must discard nothing"
    );
}

#[test]
fn lukka_plus_one_noncreature_discard_draws_one_card() {
    let (game, alice) = resolve_lukka_plus_one(CardType::Sorcery, true);
    let player = game.player(alice).expect("Alice should exist");

    assert_eq!(
        player.graveyard.len(),
        1,
        "the chosen card must be discarded"
    );
    assert_eq!(
        player.library.len(),
        3,
        "the default branch must draw one card"
    );
    assert_eq!(
        player.hand.len(),
        1,
        "one discard followed by one draw leaves one card"
    );
}

#[test]
fn lukka_plus_one_creature_discard_draws_two_instead_of_one_plus_two() {
    let (game, alice) = resolve_lukka_plus_one(CardType::Creature, true);
    let player = game.player(alice).expect("Alice should exist");

    assert_eq!(
        player.graveyard.len(),
        1,
        "the creature card must be discarded"
    );
    assert_eq!(
        player.library.len(),
        2,
        "the replacement branch must draw exactly two cards, not one plus two"
    );
    assert_eq!(
        player.hand.len(),
        2,
        "one discard followed by two draws leaves two cards"
    );
}

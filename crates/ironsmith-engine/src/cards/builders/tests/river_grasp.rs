#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::oracle_text_by_name;
use super::*;
use crate::decision::DecisionMaker;
use crate::effects::{ExecutionContext, ResolvedTarget};

const RIVER_GRASP_LINE: &str = "If {U} was spent to cast this spell, return up to one target creature to its owner's hand. If {B} was spent to cast this spell, target player reveals their hand, you choose a nonland card from it, then that player discards that card.";

fn river_grasp_definition() -> CardDefinition {
    let oracle = oracle_text_by_name()
        .get("River's Grasp")
        .expect("River's Grasp should be present in cards.json");
    assert_eq!(oracle, RIVER_GRASP_LINE);
    CardDefinitionBuilder::new(CardId::new(), "River's Grasp")
        .parse_text(format!(
            "Mana cost: {{3}}{{U/B}}\nType: Sorcery\nFirst printed set: Shadowmoor\n{oracle}"
        ))
        .expect("the authoritative metadata-backed payload should parse")
}

#[derive(Default)]
struct RiverGraspDecisions {
    chosen: Option<ObjectId>,
    legal_hand_choices: Vec<ObjectId>,
    public_reveals: Vec<(PlayerId, PlayerId, Vec<ObjectId>)>,
}

impl DecisionMaker for RiverGraspDecisions {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.legal_hand_choices = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        self.chosen
            .filter(|chosen| self.legal_hand_choices.contains(chosen))
            .into_iter()
            .collect()
    }

    fn view_cards(
        &mut self,
        _game: &crate::GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        if ctx.public {
            self.public_reveals
                .push((viewer, ctx.subject, cards.to_vec()));
        }
    }
}

fn test_card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder.power_toughness(PowerToughness::fixed(2, 2)).build()
    } else {
        builder.build()
    }
}

fn resolve_river_grasp(
    blue: u32,
    black: u32,
) -> (
    crate::GameState,
    PlayerId,
    ObjectId,
    ObjectId,
    ObjectId,
    RiverGraspDecisions,
) {
    let definition = river_grasp_definition();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature = game.create_object_from_definition(
        &test_card("River Creature", vec![CardType::Creature]),
        bob,
        Zone::Battlefield,
    );
    let nonland = game.create_object_from_definition(
        &test_card("River Nonland", vec![CardType::Sorcery]),
        bob,
        Zone::Hand,
    );
    let land = game.create_object_from_definition(
        &test_card("River Land", vec![CardType::Land]),
        bob,
        Zone::Hand,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.object_mut(spell)
        .expect("River's Grasp should exist on the stack")
        .mana_spent_to_cast = crate::player::ManaPool {
        blue,
        black,
        ..Default::default()
    };

    let mut decisions = RiverGraspDecisions {
        chosen: Some(nonland),
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new(spell, alice, &mut decisions).with_targets(vec![
        ResolvedTarget::Object(creature),
        ResolvedTarget::Player(bob),
    ]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        definition
            .spell_effect
            .as_ref()
            .expect("River's Grasp should have a spell program"),
        None,
        &[],
    )
    .expect("River's Grasp should resolve");

    (game, bob, creature, nonland, land, decisions)
}

#[test]
fn river_grasp_keeps_two_typed_mana_spent_branches_and_exact_surface() {
    let definition = river_grasp_definition();
    assert_eq!(compiled_text_lines(&definition), vec![RIVER_GRASP_LINE]);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("River's Grasp should have a spell program");
    let [blue_segment, black_segment] = program.segments.as_slice() else {
        panic!("River's Grasp should preserve its two source sentences: {program:#?}");
    };
    let [blue_effect] = blue_segment.default_effects.as_slice() else {
        panic!("River's Grasp should retain the blue conditional: {blue_segment:#?}");
    };
    let [black_effect] = black_segment.default_effects.as_slice() else {
        panic!("River's Grasp should retain the black conditional: {black_segment:#?}");
    };
    let blue = blue_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("the first branch should be a typed condition");
    let black = black_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("the second branch should be a typed condition");
    assert_eq!(
        blue.condition,
        crate::effect::Condition::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(crate::mana::ManaSymbol::Blue),
        }
    );
    assert_eq!(
        black.condition,
        crate::effect::Condition::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(crate::mana::ManaSymbol::Black),
        }
    );
    let [black_sequence] = black.if_true.as_slice() else {
        panic!("the black branch should retain its authored sequence: {black:#?}");
    };
    let sequence = black_sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the black branch should be a typed sequence");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    assert_eq!(sequence.effects.len(), 4);
}

#[test]
fn black_payment_reveals_only_that_players_hand_and_discards_the_chosen_nonland() {
    let (game, bob, creature, nonland, land, decisions) = resolve_river_grasp(0, 1);

    assert!(game.battlefield.contains(&creature));
    assert!(
        !game
            .player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&nonland)
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .graveyard
            .contains(&nonland)
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&land)
    );
    assert!(decisions.legal_hand_choices.contains(&nonland));
    assert!(
        !decisions.legal_hand_choices.contains(&land),
        "a land from the revealed hand must not be a legal choice"
    );
    assert_eq!(decisions.public_reveals.len(), game.players.len());
    for (_viewer, subject, cards) in decisions.public_reveals {
        assert_eq!(subject, bob);
        assert!(cards.contains(&nonland));
        assert!(cards.contains(&land));
    }
}

#[test]
fn absent_black_payment_is_an_executable_near_miss_for_the_hand_pipeline() {
    let (game, bob, creature, nonland, land, decisions) = resolve_river_grasp(1, 0);

    assert!(!game.battlefield.contains(&creature));
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&creature)
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&nonland)
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&land)
    );
    assert!(decisions.legal_hand_choices.is_empty());
    assert!(decisions.public_reveals.is_empty());
}

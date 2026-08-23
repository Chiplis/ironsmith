#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;

const ORACLE: &str = "Each player chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest. If {B}{R} was spent to cast this spell, you choose the permanents for each player instead.";

#[derive(Default)]
struct RecordingChoices {
    choosers: Vec<PlayerId>,
}

impl DecisionMaker for RecordingChoices {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.choosers.push(ctx.player);
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }
}

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder.power_toughness(PowerToughness::fixed(2, 2)).build()
    } else {
        builder.build()
    }
}

fn put_choice_sets_onto_battlefield(game: &mut crate::GameState, player: PlayerId, label: &str) {
    for card_type in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Planeswalker,
    ] {
        for copy in 0..2 {
            let definition = permanent(&format!("{label} {card_type:?} {copy}"), card_type);
            game.create_object_from_definition(&definition, player, Zone::Battlefield);
        }
    }
}

fn resolve_mythos(
    paid_black_red: bool,
) -> (crate::GameState, RecordingChoices, PlayerId, PlayerId) {
    let definition = parse_oracle_card_definition("Mythos of Snapdax");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    put_choice_sets_onto_battlefield(&mut game, alice, "Alice");
    put_choice_sets_onto_battlefield(&mut game, bob, "Bob");

    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    if paid_black_red {
        game.object_mut(source)
            .expect("Mythos should be on the stack")
            .mana_spent_to_cast = crate::player::ManaPool {
            black: 1,
            red: 1,
            ..Default::default()
        };
    }
    let mut decisions = RecordingChoices::default();
    let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("Mythos should have a spell program"),
        None,
        &[],
    )
    .expect("Mythos should resolve");
    drop(context);

    (game, decisions, alice, bob)
}

fn battlefield_count(game: &crate::GameState, player: PlayerId) -> usize {
    game.objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| game.controller_of_id(*id) == Some(player))
        .count()
}

#[test]
fn mythos_keeps_the_typed_chooser_replacement_and_exact_surface() {
    let definition = parse_oracle_card_definition("Mythos of Snapdax");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let debug = format!("{:#?}", definition.spell_effect);
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("chooser: IteratedPlayer"), "{debug}");
    assert!(debug.contains("chooser: You"), "{debug}");
}

#[test]
fn mythos_changes_only_who_makes_each_players_four_choices() {
    let (unpaid_game, unpaid_choices, alice, bob) = resolve_mythos(false);
    assert_eq!(battlefield_count(&unpaid_game, alice), 4);
    assert_eq!(battlefield_count(&unpaid_game, bob), 4);
    assert!(unpaid_choices.choosers.contains(&alice));
    assert!(unpaid_choices.choosers.contains(&bob));

    let (paid_game, paid_choices, alice, bob) = resolve_mythos(true);
    assert_eq!(battlefield_count(&paid_game, alice), 4);
    assert_eq!(battlefield_count(&paid_game, bob), 4);
    assert_eq!(paid_choices.choosers.len(), 8);
    assert!(
        paid_choices
            .choosers
            .iter()
            .all(|chooser| *chooser == alice),
        "the caster should make every choice when black and red were spent"
    );
}

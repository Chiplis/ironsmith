#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Kicker {2}{W}\nChoose target creature with mana value 3 or less. If this spell was kicked, instead choose target creature. Exile the chosen creature, then its controller gains life equal to its mana value.";

fn creature(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

#[test]
fn expel_keeps_one_choice_replacement_and_one_common_exile_life_tail() {
    let definition = parse_oracle_card_definition("Expel the Unworthy");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Expel must have a spell program");
    let debug = format!("{program:#?}");
    assert!(debug.contains("SelfReplacementBranch"), "{debug}");
    assert!(debug.contains("ManaValueOf"), "{debug}");
    assert!(debug.contains("ControllerOf"), "{debug}");
}

fn resolve_expelling(kicked: bool, mana_value: u8) -> (Zone, i32) {
    let definition = parse_oracle_card_definition("Expel the Unworthy");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let target = game.create_object_from_definition(
        &creature("Unworthy", mana_value),
        bob,
        Zone::Battlefield,
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::default();
    if kicked {
        paid.mark_label_paid("Kicker");
    }
    game.object_mut(spell)
        .expect("spell exists")
        .optional_costs_paid = paid.clone();
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::Target::Object(target)])
            .with_optional_costs_paid(paid),
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Expel should resolve");
    (
        game.object(target)
            .expect("target remains represented")
            .zone,
        game.player(bob).expect("Bob").life,
    )
}

#[test]
fn both_expel_target_domains_share_the_exile_and_life_followup() {
    assert_eq!(resolve_expelling(false, 2), (Zone::Exile, 22));
    assert_eq!(resolve_expelling(true, 5), (Zone::Exile, 25));
}

use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn laboratory_drudge_preserves_both_graveyard_history_branches() {
    assert_eq!(
        compiled_card_text("Laboratory Drudge"),
        "At the beginning of each end step, draw a card if you've cast a spell from a graveyard or activated an ability of a card in a graveyard this turn."
    );
}

#[test]
fn standalone_cast_and_activation_origin_history_conditions_remain_distinct() {
    let cast = CardDefinitionBuilder::new(CardId::new(), "Cast History")
        .parse_text(
            "At the beginning of your end step, draw a card if you've cast a spell from exile this turn.",
        )
        .expect("standalone cast-origin history condition should parse");
    assert_eq!(
        canonical_compiled_lines(&cast).join("\n"),
        "At the beginning of your end step, draw a card if you've cast a spell from exile this turn."
    );

    let activation = CardDefinitionBuilder::new(CardId::new(), "Activation History")
        .parse_text(
            "At the beginning of your end step, draw a card if you activated an ability of a card in your graveyard this turn.",
        )
        .expect("standalone activation-origin history condition should parse");
    assert_eq!(
        canonical_compiled_lines(&activation).join("\n"),
        "At the beginning of your end step, draw a card if you activated an ability of a card in a graveyard this turn."
    );
}

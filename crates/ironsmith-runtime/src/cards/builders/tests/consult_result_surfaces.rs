use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn compiled_card_text(name: &str) -> String {
    assert_oracle_card_parses_strict(name);
    canonical_compiled_lines(&parse_oracle_card_definition(name)).join("\n")
}

#[test]
fn named_consult_result_surfaces_preserve_typed_alternatives_and_remainders() {
    let gamekeeper = compiled_card_text("Gamekeeper");
    assert!(
        gamekeeper.contains(
            "If you do, reveal cards from the top of your library until you reveal a creature card. Put that card onto the battlefield and put all other cards revealed this way into your graveyard"
        ),
        "{gamekeeper}"
    );

    let illuna = compiled_card_text("Illuna, Apex of Wishes");
    assert!(
        illuna.contains(
            "exile cards from the top of your library until you exile a nonland permanent card. You may put it onto the battlefield. If you don't, put it into its owner's hand"
        ),
        "{illuna}"
    );

    let ryan = compiled_card_text("Ryan Sinclair");
    assert!(
        ryan.contains(
            "exile cards from the top of your library until you exile a nonland card. If its mana value is less than or equal to Ryan's power, You may cast it without paying its mana cost. Put the exiled cards on the bottom of your library in a random order"
        ),
        "{ryan}"
    );

    let solstice = compiled_card_text("Solstice Revelations");
    assert!(
        solstice.contains(
            "Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than the number of Mountains you control. If you don't cast that card this way, put it into your hand"
        ),
        "{solstice}"
    );

    let songbirds = compiled_card_text("Songbirds' Blessing");
    assert!(
        songbirds.contains(
            "reveal cards from the top of your library until you reveal an Aura card. You may put that card onto the battlefield. If you don't, put it into your hand. Put the rest on the bottom of your library in a random order"
        ),
        "{songbirds}"
    );
}

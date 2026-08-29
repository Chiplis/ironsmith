use super::*;
use crate::lexer::lex_line;

#[test]
fn energy_for_each_keeps_for_each_value_surface() {
    let tokens =
        lex_line("get {E} for each creature attacking you.", 0).expect("energy clause should lex");
    let effect = parse_get(&tokens, None).expect("energy clause should parse");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("EnergyCounters"), "{debug}");
    assert!(debug.contains("ForEach"), "{debug}");
    assert!(debug.contains("attacking_player_only: true"), "{debug}");
}

use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_component_choice_and_source_shapes() {
    assert_eq!(
        classify_granted_ability_surface(
            &lex_line("can't be blocked except by creatures with haste", 0).unwrap()
        ),
        GrantedAbilitySurface::CantBeBlockedExceptByHaste
    );
    assert!(matches!(
        classify_granted_ability_surface(&lex_line("hexproof from red", 0).unwrap()),
        GrantedAbilitySurface::HexproofFrom {
            filter_start_token: 2
        }
    ));
    assert_eq!(
        parse_ability_choice_shape(&lex_line("your choice of flying or vigilance", 0).unwrap())
            .unwrap()
            .options
            .len(),
        2
    );
    assert_eq!(
        parse_ability_choice_shape(&lex_line("flying, first strike, or trample", 0).unwrap())
            .unwrap()
            .options
            .len(),
        3
    );
    let source_tokens =
        lex_line("This creature gains {T}: Draw a card until end of turn.", 0).unwrap();
    let source = parse_source_gain_ability_shape(&source_tokens).unwrap();
    assert_eq!(source.duration, Until::EndOfTurn);
    assert!(!source.ability_tokens.is_empty());

    let life_gain = lex_line(
        "Target player sacrifices a creature, then gains life equal to that creature's toughness.",
        0,
    )
    .unwrap();
    assert!(parse_simple_gain_ability_shape(&life_gain).is_none());
}

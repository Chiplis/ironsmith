use super::*;

#[test]
pub(super) fn parses_all_unspent_mana_resource_shape() {
    assert!(parse_resource_all_unspent_mana_shape(&lex(
        "all unspent mana"
    )));
    assert!(!parse_resource_all_unspent_mana_shape(&lex(
        "all unspent energy"
    )));
}

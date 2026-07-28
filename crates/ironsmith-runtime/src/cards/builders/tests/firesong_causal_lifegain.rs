use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};

#[test]
fn firesong_keeps_shared_spell_qualifiers_and_causal_life_gain() {
    assert_oracle_card_parses_strict("Firesong and Sunspeaker");
    let definition = parse_oracle_card_definition("Firesong and Sunspeaker");
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");

    assert_eq!(
        rendered,
        "Red instant and sorcery spells you control have lifelink.\n\
         Whenever a white instant or sorcery spell causes you to gain life, Firesong and Sunspeaker deals 3 damage to target creature or player."
    );
}

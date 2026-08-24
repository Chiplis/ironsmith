use super::*;

#[test]
pub(super) fn quoted_emblem_payload_does_not_consume_an_unquoted_followup() {
    let kiora_ultimate = lex_line(
        r#"You get an emblem with "Whenever a creature you control enters, you may have it fight target creature." Then create three 8/8 blue Octopus creature tokens."#,
        0,
    )
    .unwrap();

    assert!(
        parse_emblem_payload_tokens(&kiora_ultimate).is_none(),
        "the whole-sentence emblem guard must leave Kiora's unquoted token-creation tail for ordinary sentence dispatch"
    );
}

#[test]
pub(super) fn quoted_emblem_payload_accepts_only_a_synthetic_outer_period() {
    let grouped_statement = lex_line(
        r#"You get an emblem with "You have no maximum hand size."."#,
        0,
    )
    .unwrap();
    let parsed = parse_emblem_payload_tokens(&grouped_statement)
        .expect("statement grouping's outer period should not hide the quoted emblem");

    assert_eq!(parsed.ability_groups.len(), 1);
    assert!(parsed.requires_whole_sentence_dispatch);
    assert_eq!(
        render_token_slice(parsed.ability_groups[0]),
        "You have no maximum hand size."
    );

    let unquoted_tail = lex_line(
        r#"You get an emblem with "You have no maximum hand size.". Then draw a card."#,
        0,
    )
    .unwrap();
    assert!(
        parse_emblem_payload_tokens(&unquoted_tail).is_none(),
        "the optional outer period must not authorize an unquoted continuation"
    );
}

use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("permission subject fixture should lex")
}

#[test]
fn parses_typed_spell_subject_facts() {
    let singular = parse_spell_subject_facts(&lex("a spell you own"));
    assert!(singular.contains_singular_spell);
    assert!(!singular.contains_plural_spells);
    assert!(singular.starts_with_generic_spell);

    assert_eq!(
        parse_exact_permission_subject(&lex("permanent spells")),
        Some(ExactPermissionSubject::PermanentSpells)
    );
}

#[test]
fn parses_type_lists_and_binary_subjects() {
    let types = parse_simple_spell_type_list_filter_tokens(&lex(
        "artifact, creature, or enchantment spells",
    ))
    .expect("type list");
    assert_eq!(
        types.card_types,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment
        ]
    );

    let binary = parse_permission_subject_filter_tokens(&lex("artifact cards or creature cards"))
        .expect("filter parse")
        .expect("binary filter");
    assert_eq!(binary.any_of.len(), 2);
}

#[test]
fn parses_special_permission_subject_filters() {
    let aura = parse_permission_subject_filter_tokens(&lex("Aura spells with enchant creature"))
        .expect("filter parse")
        .expect("aura filter");
    assert!(aura.subtypes.contains(&Subtype::Aura));
    assert_eq!(aura.ability_markers, ["enchant creature"]);

    let permanent = parse_permission_subject_filter_tokens(&lex("permanent spells"))
        .expect("filter parse")
        .expect("permanent filter");
    assert!(permanent.card_types.contains(&CardType::Creature));
    assert!(permanent.card_types.contains(&CardType::Planeswalker));

    let noncreature = parse_cast_permission_filter_tokens(&lex("noncreature spells"))
        .expect("filter parse")
        .expect("noncreature-spell filter");
    assert!(
        noncreature
            .excluded_card_types
            .contains(&CardType::Creature)
    );
    assert!(noncreature.excluded_card_types.contains(&CardType::Land));
}

#[test]
fn qualified_generic_spell_subjects_keep_keyword_constraints() {
    let cycling = parse_cast_permission_filter_tokens(&lex("spells that have a cycling ability"))
        .expect("qualified spell subject should not hard-error")
        .expect("qualified spell subject should produce a filter");
    assert_eq!(cycling.ability_markers, vec!["cycling".to_string()]);

    let generic = parse_cast_permission_filter_tokens(&lex("spells"))
        .expect("generic spell subject should not hard-error")
        .expect("generic spell subject should produce a filter");
    assert_eq!(generic, ObjectFilter::default());
}

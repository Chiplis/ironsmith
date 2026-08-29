use super::*;
use crate::lexer::{lex_line, parser_token_word_refs};

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).expect("lex cant structure fixture")
}

#[test]
fn captures_multi_sentence_decline_boundaries() {
    let tokens = lex("Damage can't be prevented this turn. This deals 2 damage to any target.");
    let parsed = parse_multi_sentence_cant_decline_tokens(&tokens).unwrap();
    assert_eq!(
        parser_token_word_refs(parsed.first_sentence_tokens),
        ["damage", "cant", "be", "prevented", "this", "turn"]
    );
    assert_eq!(
        parser_token_word_refs(parsed.remaining_sentence_tokens),
        ["this", "deals", "2", "damage", "to", "any", "target"]
    );
    assert!(parse_multi_sentence_cant_decline_tokens(&lex("Players can't gain life.")).is_none());
}

#[test]
fn captures_direct_temporary_cast_decline_table() {
    let cases = [
        (
            "Your opponents can't cast spells this turn.",
            DirectTemporaryCastSubject::YourOpponents,
        ),
        (
            "Each opponent cannot cast creature spells this turn.",
            DirectTemporaryCastSubject::EachOpponent,
        ),
        (
            "Each player can't cast more than one spell this turn.",
            DirectTemporaryCastSubject::EachPlayer,
        ),
        (
            "Players can't cast spells this turn.",
            DirectTemporaryCastSubject::Players,
        ),
        (
            "Target player can't cast noncreature spells this turn.",
            DirectTemporaryCastSubject::TargetPlayer,
        ),
        (
            "You can't cast spells from exile this turn.",
            DirectTemporaryCastSubject::You,
        ),
    ];
    for (raw, expected) in cases {
        let tokens = lex(raw);
        let parsed = parse_direct_temporary_cast_decline_tokens(&tokens)
            .unwrap_or_else(|| panic!("fixture did not parse: {raw}"));
        assert_eq!(parsed.subject, expected, "fixture: {raw}");
        assert_eq!(
            parser_token_word_refs(parsed.duration_tokens),
            ["this", "turn"]
        );
        assert!(!parsed.spell_descriptor_tokens.is_empty());
    }
    for raw in [
        "Your opponents can't cast spells.",
        "Your opponents can't cast spells unless they pay {2} this turn.",
        "Each opponent who lost life can't cast spells this turn.",
        "Creatures can't cast spells this turn.",
    ] {
        assert!(
            parse_direct_temporary_cast_decline_tokens(&lex(raw)).is_none(),
            "near miss: {raw}"
        );
    }
}

#[test]
fn captures_iterated_player_and_leading_if_declines() {
    let cases = [
        (
            "Each opponent who lost life can't block.",
            IteratedPlayerLead::Each,
            IteratedPlayerScope::Opponent,
        ),
        (
            "Each players who drew a card can't attack.",
            IteratedPlayerLead::Each,
            IteratedPlayerScope::Player,
        ),
        (
            "For each opponent who does, draw a card.",
            IteratedPlayerLead::ForEach,
            IteratedPlayerScope::Opponent,
        ),
        (
            "For each player who discarded, create a token.",
            IteratedPlayerLead::ForEach,
            IteratedPlayerScope::Player,
        ),
    ];
    for (raw, lead, player) in cases {
        let tokens = lex(raw);
        let parsed = parse_iterated_player_who_decline_tokens(&tokens)
            .unwrap_or_else(|| panic!("fixture did not parse: {raw}"));
        assert_eq!((parsed.lead, parsed.player), (lead, player));
        assert!(!parsed.predicate_tokens.is_empty());
    }
    let tokens = lex("If a creature would attack, it can't block.");
    let parsed = parse_leading_if_cant_decline_tokens(&tokens).unwrap();
    assert_eq!(parser_token_word_refs(parsed.if_tokens), ["if"]);
    assert!(parse_leading_if_cant_decline_tokens(&lex("Players can't gain life.")).is_none());
}

#[test]
fn captures_stat_modifier_conjunction_decline() {
    let tokens = lex("Enchanted creature gets +2/+2 and can't be blocked.");
    let parsed = parse_stat_modifier_conjunction_decline_tokens(&tokens).unwrap();
    assert_eq!(parsed.verb, StatModifierVerb::Gets);
    assert_eq!(
        parser_token_word_refs(parsed.subject_tokens),
        ["enchanted", "creature"]
    );
    assert_eq!(parser_token_word_refs(parsed.modifier_tokens), ["+2/+2"]);
    assert_eq!(parser_token_word_refs(parsed.negation_tokens), ["cant"]);
    for raw in [
        "Enchanted creature gets +2/+2.",
        "Enchanted creature has flying and can't be blocked.",
        "Enchanted creature can't be blocked and gets +2/+2.",
    ] {
        assert!(
            parse_stat_modifier_conjunction_decline_tokens(&lex(raw)).is_none(),
            "near miss: {raw}"
        );
    }
}

#[test]
fn expands_inherited_negation_and_subject_conjunctions() {
    let tokens = lex("Creatures you control and artifacts you control can't be sacrificed.");
    let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
    assert_eq!(expanded.negated_anchor, 1);
    assert_eq!(expanded.segments.len(), 2);
    assert_eq!(
        parser_token_word_refs(&expanded.segments[0]),
        ["creatures", "you", "control", "cant", "be", "sacrificed"]
    );

    let tokens = lex("Players can't gain life and can't search libraries.");
    let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
    assert_eq!(expanded.negated_anchor, 0);
    assert_eq!(
        parser_token_word_refs(&expanded.segments[1]),
        ["players", "cant", "search", "libraries"]
    );

    let tokens = lex("Players can't gain life and draw cards.");
    let expanded = parse_cant_conjunction_expansion_tokens(&tokens).unwrap();
    assert_eq!(expanded.segments.len(), 1);

    let inherited_subject_cases: &[(&str, &[&str])] = &[
        (
            "Creatures your opponents control can't block, and they can't attack you.",
            &["creatures", "your", "opponents", "control"],
        ),
        (
            "This creature can't block, and it can't attack.",
            &["this", "creature"],
        ),
    ];
    for (text, expected_subject) in inherited_subject_cases {
        let expanded = parse_cant_conjunction_expansion_tokens(&lex(text)).unwrap();
        assert_eq!(expanded.segments.len(), 2, "{text}");
        let second = parser_token_word_refs(&expanded.segments[1]);
        let expected = expected_subject
            .iter()
            .copied()
            .chain(["cant", "attack"])
            .collect::<Vec<_>>();
        assert!(
            second.starts_with(&expected),
            "inherited subject was not expanded for {text}: {second:?}"
        );
    }
}

#[test]
fn captures_generic_block_transform_and_untap_actions() {
    let block = lex("This creature can't block creatures with flying.");
    let GenericNegatedCantAction::SourceBlocksAttacker {
        source,
        attacker_tokens,
        ..
    } = parse_generic_negated_cant_action_tokens(&block).unwrap()
    else {
        panic!("expected source-block action");
    };
    assert_eq!(source, SourceCantSubject::ThisCreature);
    assert_eq!(
        parser_token_word_refs(attacker_tokens),
        ["creatures", "with", "flying"]
    );

    let transform = lex("Non-Human creatures can't transform.");
    let GenericNegatedCantAction::SubjectCantTransform { subject_tokens, .. } =
        parse_generic_negated_cant_action_tokens(&transform).unwrap()
    else {
        panic!("expected transform action");
    };
    assert_eq!(
        parser_token_word_refs(subject_tokens),
        ["non", "human", "creatures"]
    );

    let untap = lex("Target creature can't untap during its controller's next untap step.");
    let parsed = parse_negated_untap_remainder_tokens(&untap).unwrap();
    assert_eq!(
        parser_token_word_refs(parsed.subject_tokens),
        ["target", "creature"]
    );
    assert_eq!(
        parser_token_word_refs(parsed.post_untap_tokens),
        ["during", "its", "controllers", "next", "untap", "step"]
    );

    for raw in [
        "This creature can't block.",
        "This creature can block creatures with flying.",
        "Non-Human creatures can transform.",
        "Target creature can't attack.",
    ] {
        assert!(
            parse_generic_negated_cant_action_tokens(&lex(raw)).is_none(),
            "near miss: {raw}"
        );
    }
}

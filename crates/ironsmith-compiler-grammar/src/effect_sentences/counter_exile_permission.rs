//! One counter instruction owns its destination replacement and the priced
//! permission on the exact object its successful counter places in exile.
//!
//! Do not enable the generic free-cost + while-exiled tail here: an unrelated
//! tagged permission still has a different lifetime/identity contract.

use crate::cards::builders::{CardTextError, EffectAst, OwnedLexToken, TargetAst};
use crate::lexer::{TokenKind, split_lexed_sentences};
use ironsmith_core::{CounterExileGate, CounterExilePermission};

fn exact(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
    tokens.len() == words.len()
        && tokens.iter().zip(words).all(|(token, word)| {
            token.parser_text() == *word
                && token.kind == if *word == "," { TokenKind::Comma } else { TokenKind::Word }
        })
}

pub(crate) fn has_permanent_counter_exile_gate(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    words.windows(8).any(|part| part == [
        "if", "a", "permanent", "spell", "is", "countered", "this", "way",
    ]) && words.contains(&"exile")
}

pub(crate) fn is_candidate(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    // Trigger recognition owns the header. This production reads only the
    // complete post-comma effect body handed to it by that owner.
    if words.first().is_some_and(|word| matches!(*word, "when" | "whenever")) {
        return false;
    }
    let contains = |phrase: &[&str]| words.windows(phrase.len()).any(|part| part == phrase);
    let counter_reference = contains(&["countered", "this", "way"]);
    let durable_tail = contains(&["for", "as", "long", "as"])
        || contains(&["remains", "exiled"]);
    let cast_or_play_tail = words.iter().any(|word| matches!(*word, "cast" | "play"));
    // The legacy two-sentence owner cannot represent a permanent-only gate.
    // Do not allow deleting the permission to erase that authored condition.
    // Ordinary unconditional two-sentence exile counters remain outside this owner. Once
    // a permission is authored, however, a missing/garbled lifetime cannot
    // fall back to the lossy generic counter-destination recognizer.
    has_permanent_counter_exile_gate(tokens)
        || (counter_reference && cast_or_play_tail && words.contains(&"exile"))
        || (durable_tail
            && (counter_reference || words.first().is_some_and(|word| *word == "counter")))
}

pub(crate) fn parse(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !is_candidate(tokens) {
        return Ok(None);
    }

    // Commit to the entire related document. Falling through after recognizing
    // only marker words would erase a gate, accept a dangling reference, or
    // silently discard part of the price/lifetime clause.
    let unsupported = || CardTextError::ParseError(
        "counter exile permission requires the complete counter, destination replacement, and exact free-cost while-exiled permission".into(),
    );
    let sentences = split_lexed_sentences(tokens);
    let [counter, replacement, permission] = sentences.as_slice() else {
        return Err(unsupported());
    };
    if !exact(counter, &["counter", "target", "spell"]) {
        return Err(unsupported());
    }

    // Token comparisons, including the comma and possessive, consume the
    // full production rather than a bag of words.
    let (gate, remainder) = if replacement.len() >= 3
        && exact(&replacement[..3], &["if", "that", "spell"])
    {
        (CounterExileGate::AnySpell, &replacement[3..])
    } else if replacement.len() >= 4
        && exact(&replacement[..4], &["if", "a", "permanent", "spell"])
    {
        (CounterExileGate::PermanentSpell, &replacement[4..])
    } else {
        return Err(unsupported());
    };
    if !exact(remainder, &[
        "is", "countered", "this", "way", ",", "exile", "it", "instead", "of",
        "putting", "it", "into", "its", "owner's", "graveyard",
    ]) {
        return Err(unsupported());
    }

    let (allow_land, tail) = if permission.len() >= 4
        && exact(&permission[..4], &["you", "may", "play", "it"])
    {
        (true, &permission[4..])
    } else if permission.len() >= 5
        && exact(&permission[..5], &["you", "may", "cast", "that", "card"])
    {
        (false, &permission[5..])
    } else {
        return Err(unsupported());
    };
    if !exact(tail, &[
        "without", "paying", "its", "mana", "cost", "for", "as", "long", "as",
        "it", "remains", "exiled",
    ]) {
        return Err(unsupported());
    }
    Ok(Some(vec![EffectAst::subject_verb_counter_with_exile_permission(
        TargetAst::Spell(crate::util::span_from_tokens(&counter[1..])),
        CounterExilePermission { gate, allow_land },
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{PlayerAst, StackActionAst, SubjectVerbActionAst};
    use crate::lexer::lex_line;

    const COUNTER: &str = "Counter target spell.";
    const REPLACEMENT: &str = "If that spell is countered this way, exile it instead of putting it into its owner's graveyard.";
    const PERMANENT: &str = "If a permanent spell is countered this way, exile it instead of putting it into its owner's graveyard.";
    const CAST: &str = "You may cast that card without paying its mana cost for as long as it remains exiled.";
    const PLAY: &str = "You may play it without paying its mana cost for as long as it remains exiled.";

    #[test]
    fn complete_counter_program_preserves_gate_actor_and_play_domain() {
        for (replacement, permission, gate, allow_land) in [
            (REPLACEMENT, PLAY, CounterExileGate::AnySpell, true),
            (REPLACEMENT, CAST, CounterExileGate::AnySpell, false),
            (PERMANENT, CAST, CounterExileGate::PermanentSpell, false),
        ] {
            let tokens = lex_line(&format!("{COUNTER} {replacement} {permission}"), 0).unwrap();
            let effects = super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens)
                .expect("the full counter program should parse");
            let [EffectAst::SubjectVerb(subject)] = effects.as_slice() else {
                panic!("one owner is required, without a may prompt or separate grant: {effects:#?}");
            };
            assert_eq!(subject.subject.player, PlayerAst::You);
            let SubjectVerbActionAst::Stack(StackActionAst::Counter {
                target: TargetAst::Spell(Some(_)),
                exile_permission: Some(permission),
            }) = &subject.action else {
                panic!("all spells remain targetable, with a rider-owned gate: {effects:#?}");
            };
            assert_eq!(permission.gate, gate);
            assert_eq!(permission.allow_land, allow_land);
        }
    }

    #[test]
    fn incomplete_or_unsupported_counter_programs_cannot_fall_back() {
        let valid = format!("{COUNTER} {REPLACEMENT} {CAST}");
        let near_misses = [
            format!("{REPLACEMENT} {CAST}"),
            format!("{COUNTER} {CAST}"),
            format!("Draw a card. {REPLACEMENT} {CAST}"),
            format!("{valid} Draw a card."),
            valid.replace("a permanent spell", "a creature spell")
                .replace("that spell is countered", "a creature spell is countered"),
            valid.replace("exile it instead of putting it into its owner's graveyard", "exile it"),
            valid.replace("instead of putting it into", "after putting it into"),
            valid.replace("You may", "Its owner may"),
            valid.replace("cast that card", "cast a copy of that card"),
            valid.replace("cast that card", "cast cards from exile"),
            valid.replace("cast that card", "cast a card from the chosen pile"),
            valid.replace("without paying its mana cost", "without paying its mana cost and draw a card"),
            valid.replace("it remains exiled", "you control this creature"),
            valid.replace("remains exiled.", "remains exiled during your turn."),
            valid.replace("remains exiled.", "remains exiled {1}."),
            valid.replace(" for as long as it remains exiled", ""),
            valid.replace("for as long as it remains exiled", "until your next turn"),
            valid.replace("for as long as it remains exiled", "while you control this creature"),
            valid.replace("Counter target spell.", "Counter target permanent spell."),
        ];
        for text in near_misses {
            let tokens = lex_line(&text, 0).unwrap();
            assert!(
                super::super::dispatch_entry::parse_effect_sentences_lexed(&tokens).is_err(),
                "a partial or mismatched program was accepted: {text}",
            );
        }
    }

    #[test]
    fn unrelated_permission_families_are_not_claimed_by_this_owner() {
        for text in [
            "You may cast it without paying its mana cost.",
            "You may play that card for as long as it remains exiled.",
            "You may cast that card without paying its mana cost this turn.",
            "Exile target nonland permanent. For as long as that card remains exiled, its owner may cast it without paying its mana cost.",
            CAST,
        ] {
            assert!(parse(&lex_line(text, 0).unwrap()).unwrap().is_none(), "{text}");
        }
    }

    #[test]
    fn trigger_header_is_owned_by_trigger_parser_and_body_is_kept_correlated() {
        let body = format!("{COUNTER} {REPLACEMENT} {CAST}");
        let full = lex_line(&format!("When this creature is turned face up, {body}"), 0).unwrap();
        assert!(parse(&full).unwrap().is_none());
        let body_tokens = lex_line(&body, 0).unwrap();
        assert!(crate::semantic_line_parsing::is_exact_correlated_trigger_effect_bundle(&body_tokens));
        assert!(parse(&body_tokens).unwrap().is_some());
        let malformed = lex_line(&body.replace(" for as long as it remains exiled", ""), 0).unwrap();
        assert!(crate::semantic_line_parsing::is_exact_correlated_trigger_effect_bundle(&malformed));
        assert!(parse(&malformed).is_err(), "grouping must preserve a committed diagnostic");
    }

    #[test]
    fn deleted_permission_cannot_expose_the_ungated_permanent_fallback() {
        let gated = lex_line(&format!("{COUNTER} {PERMANENT}"), 0).unwrap();
        assert!(super::super::dispatch_entry::parse_effect_sentences_lexed(&gated).is_err());
        let rider = lex_line(PERMANENT, 0).unwrap();
        assert!(super::super::dispatch_entry::future_zone_replacement_from_sentence_tokens(&rider).is_none());

        let ordinary = lex_line(&format!("{COUNTER} {REPLACEMENT}"), 0).unwrap();
        assert!(parse(&ordinary).unwrap().is_none());
        assert!(super::super::dispatch_entry::parse_effect_sentences_lexed(&ordinary).is_ok());
        let ordinary_rider = lex_line(REPLACEMENT, 0).unwrap();
        assert!(super::super::dispatch_entry::future_zone_replacement_from_sentence_tokens(&ordinary_rider).is_some());
    }
}

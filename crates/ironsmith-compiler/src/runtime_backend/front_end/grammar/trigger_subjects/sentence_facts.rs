use crate::runtime_backend::lexer::{OwnedLexToken, TokenKind, parser_token_word_refs};

use super::{exact_word_occurs, parse_trigger_word_token, word_slice_has_any_prefix};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerSentenceSurfaceFacts {
    pub(crate) spawn_scion_mana_reminder: bool,
    pub(crate) round_up_each_time: bool,
}

pub(crate) fn parse_trigger_sentence_surface_facts(
    tokens: &[OwnedLexToken],
) -> TriggerSentenceSurfaceFacts {
    let words = parser_token_word_refs(tokens);
    let spawn_scion_mana_reminder = word_slice_has_any_prefix(
        &words,
        &[
            &["they", "have"],
            &["it", "has"],
            &["this", "token", "has"],
            &["those", "tokens", "have"],
        ],
    ) && ["sacrifice", "add", "c"]
        .iter()
        .all(|word| exact_word_occurs(&words, &[*word]));
    TriggerSentenceSurfaceFacts {
        spawn_scion_mana_reminder,
        round_up_each_time: word_slice_has_any_prefix(&words, &[&["round", "up", "each", "time"]]),
    }
}

pub(crate) fn parse_embedded_token_rules_boundary_tokens(
    tokens: &[OwnedLexToken],
) -> Option<usize> {
    let words = parser_token_word_refs(tokens);
    if !exact_word_occurs(&words, &["create"]) || !exact_word_occurs(&words, &["token"]) {
        return None;
    }
    let with_index = parse_trigger_word_token(tokens, &["with"])?;
    let starts_tap_ability = tokens
        .get(with_index + 1)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "t");
    let starts_quoted_rule = tokens
        .get(with_index + 1)
        .is_some_and(|token| token.kind == TokenKind::Quote)
        && tokens[with_index + 2..]
            .iter()
            .any(|token| token.kind == TokenKind::Quote);
    (starts_tap_ability || starts_quoted_rule).then_some(with_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn typed_sentence_facts_preserve_token_reminder_and_round_up_prefixes() {
        let reminder = lex_line("They have sacrifice this token add c", 0).unwrap();
        assert!(parse_trigger_sentence_surface_facts(&reminder).spawn_scion_mana_reminder);

        let round_up = lex_line("Round up each time you divide damage", 0).unwrap();
        assert!(parse_trigger_sentence_surface_facts(&round_up).round_up_each_time);
    }

    #[test]
    fn typed_embedded_token_rule_facts_return_token_boundary() {
        let tokens = lex_line("Create a 1/1 token with t add c", 0).unwrap();
        let boundary = parse_embedded_token_rules_boundary_tokens(&tokens).unwrap();
        assert_eq!(tokens[boundary].as_word(), Some("with"));

        let quoted = lex_line(
            "Create two 0/2 blue Illusion creature tokens with \"Whenever this token blocks a creature, that creature doesn't untap during its controller's next untap step.\"",
            0,
        )
        .unwrap();
        let boundary = parse_embedded_token_rules_boundary_tokens(&quoted).unwrap();
        assert_eq!(quoted[boundary].as_word(), Some("with"));

        let unrelated = lex_line("Target token gains flying", 0).unwrap();
        assert!(parse_embedded_token_rules_boundary_tokens(&unrelated).is_none());

        let ordinary_modifier = lex_line("Create a 1/1 token with flying", 0).unwrap();
        assert!(parse_embedded_token_rules_boundary_tokens(&ordinary_modifier).is_none());
    }
}

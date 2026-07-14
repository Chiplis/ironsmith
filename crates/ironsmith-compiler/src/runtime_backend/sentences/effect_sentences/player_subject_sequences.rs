use super::super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::lex_chain_helpers::find_verb_lexed;
use crate::cards::builders::{EffectAst, PlayerAst, TagKey};
use crate::effect::Value;
use crate::target::ObjectFilter;

/// Parse a coordinated instruction in which the controller and defending
/// player each choose between discarding and sacrificing.
///
/// The outer `TagAffected` is intentionally shared by both players and both
/// branches.  Follow-ups such as "for each land card put into a graveyard this
/// way" can therefore count the objects that actually moved, regardless of
/// which action each player chose.
pub(super) fn parse_controller_and_defending_player_discard_or_sacrifice(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if token_word_refs(tokens)
        != [
            "you",
            "and",
            "defending",
            "player",
            "each",
            "discard",
            "a",
            "card",
            "or",
            "sacrifice",
            "a",
            "permanent",
        ]
    {
        return None;
    }

    fn player_choice(player: PlayerAst) -> EffectAst {
        EffectAst::UnlessAction {
            effects: vec![EffectAst::subject_verb_discard(
                player,
                Value::Fixed(1),
                false,
                false,
                None,
                None,
            )],
            alternative: vec![EffectAst::subject_verb_sacrifice(
                player,
                ObjectFilter::permanent(),
                1,
                None,
            )],
            player,
        }
    }

    let moved_tag = TagKey::from("joint_discard_or_sacrifice");
    Some(vec![EffectAst::TagAffected {
        effect: Box::new(EffectAst::Coordinated {
            effects: vec![
                player_choice(PlayerAst::You),
                player_choice(PlayerAst::Defending),
            ],
            leading_duration: false,
        }),
        tag: moved_tag,
    }])
}

/// Split a coordinated sequence whose quantified-opponent action is followed
/// by one or more explicitly controller-scoped actions.
///
/// The ordinary chain splitter intentionally requires a bare verb immediately
/// after a comma. That keeps noun lists intact, but it also leaves sequences
/// such as `each opponent discards ..., you draw ..., and you gain ...` as one
/// unparseable clause. On a triggered line, the trigger/effect probe can then
/// absorb the opponent action into the trigger and silently lose it. This
/// recognizer is deliberately narrower: the first clause must be headed by
/// `each opponent` (or `for each opponent`) and every split boundary must begin
/// an explicit `you <effect verb>` clause.
pub(super) fn split_quantified_opponent_then_controller_clauses(
    tokens: &[OwnedLexToken],
) -> Option<Vec<&[OwnedLexToken]>> {
    if !starts_quantified_opponent_action(tokens) {
        return None;
    }

    let mut clauses = Vec::new();
    let mut clause_start = 0usize;
    let mut inside_quotes = false;

    for (idx, token) in tokens.iter().enumerate() {
        if idx < clause_start {
            continue;
        }
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }

        let boundary_start = if token.kind == TokenKind::Comma {
            idx + 1
        } else if token_word_refs(std::slice::from_ref(token)).as_slice() == ["and"] {
            idx + 1
        } else {
            continue;
        };
        let Some((next_start, tail)) = controller_action_after_boundary(tokens, boundary_start)
        else {
            continue;
        };
        let current = trim_lexed_commas(tokens.get(clause_start..idx).unwrap_or_default());
        if current.is_empty() || tail.is_empty() {
            return None;
        }
        clauses.push(current);
        clause_start = next_start;
    }

    if clauses.is_empty() {
        return None;
    }
    let tail = trim_lexed_commas(tokens.get(clause_start..).unwrap_or_default());
    if !starts_controller_action(tail) {
        return None;
    }
    clauses.push(tail);
    Some(clauses)
}

fn starts_quantified_opponent_action(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    let expected_verb_idx = match words.as_slice() {
        ["each", "opponent" | "opponents", ..] => 2,
        ["for", "each", "opponent" | "opponents", ..] => 3,
        _ => return false,
    };
    find_verb_lexed(tokens).is_some_and(|(_, verb_idx)| verb_idx == expected_verb_idx)
}

fn starts_controller_action(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    matches!(words.as_slice(), ["you", ..])
        && find_verb_lexed(tokens).is_some_and(|(_, verb_idx)| verb_idx == 1)
}

fn controller_action_after_boundary(
    tokens: &[OwnedLexToken],
    mut start: usize,
) -> Option<(usize, &[OwnedLexToken])> {
    let mut tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    if token_word_refs(tail).first().copied() == Some("and") {
        start = start.saturating_add(1);
        tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    }
    starts_controller_action(tail).then_some((start, tail))
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{EffectAst, SubjectVerbActionAst, SubjectVerbEffectAst};
    use crate::runtime_backend::lexer::lex_line;

    use super::super::parse_effect_sentence_lexed;
    use super::split_quantified_opponent_then_controller_clauses;

    #[test]
    fn splits_quantified_opponent_then_explicit_controller_actions() {
        let tokens = lex_line(
            "Each opponent discards a card, you draw a card, and you gain 2 life.",
            0,
        )
        .expect("player sequence should lex");
        let clauses = split_quantified_opponent_then_controller_clauses(&tokens)
            .expect("quantified-player sequence should split");
        assert_eq!(clauses.len(), 3);
        assert_eq!(
            super::token_word_refs(clauses[0]),
            ["each", "opponent", "discards", "a", "card"]
        );
        assert_eq!(
            super::token_word_refs(clauses[1]),
            ["you", "draw", "a", "card"]
        );
        assert_eq!(
            super::token_word_refs(clauses[2]),
            ["you", "gain", "2", "life"]
        );
    }

    #[test]
    fn splits_and_coordinated_quantified_opponent_then_controller_action() {
        let tokens = lex_line(
            "Each opponent sacrifices a creature of their choice and you return a creature card from your graveyard to your hand.",
            0,
        )
        .expect("player sequence should lex");
        let clauses = split_quantified_opponent_then_controller_clauses(&tokens)
            .expect("quantified-player sequence should split");
        assert_eq!(clauses.len(), 2);
        assert_eq!(
            super::token_word_refs(clauses[0]),
            [
                "each",
                "opponent",
                "sacrifices",
                "a",
                "creature",
                "of",
                "their",
                "choice"
            ]
        );
        assert_eq!(
            super::token_word_refs(clauses[1]),
            [
                "you",
                "return",
                "a",
                "creature",
                "card",
                "from",
                "your",
                "graveyard",
                "to",
                "your",
                "hand"
            ]
        );
    }

    #[test]
    fn parses_all_actions_without_absorbing_opponent_action() {
        let tokens = lex_line(
            "Each opponent discards a card, you draw a card, and you gain 2 life.",
            0,
        )
        .expect("player sequence should lex");
        let effects = parse_effect_sentence_lexed(&tokens).expect("player sequence should parse");
        assert!(
            matches!(effects.first(), Some(EffectAst::ForEachOpponent { .. })),
            "{effects:#?}"
        );
        assert!(
            matches!(
                effects.get(1),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Draw { .. },
                    ..
                }))
            ),
            "{effects:#?}"
        );
        assert!(
            matches!(
                effects.get(2),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GainLife { .. },
                    ..
                }))
            ),
            "{effects:#?}"
        );
    }
}

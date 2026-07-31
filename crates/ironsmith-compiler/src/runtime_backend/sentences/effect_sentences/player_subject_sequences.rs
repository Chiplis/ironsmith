use super::super::lexer::{
    OwnedLexToken, TokenKind, parser_token_word_refs, token_word_refs, trim_lexed_commas,
};
use super::lex_chain_helpers::find_verb_lexed;
use crate::cards::builders::{
    EffectAst, PlayerAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst, TagKey,
    TargetAst, TextSpan,
};
use crate::effect::Value;
use crate::target::ObjectFilter;
use crate::zone::Zone;

fn comma_then_segments(tokens: &[OwnedLexToken]) -> Option<Vec<&[OwnedLexToken]>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;
    while idx + 1 < tokens.len() {
        if tokens[idx].kind == TokenKind::Comma && tokens[idx + 1].is_word("then") {
            let segment = trim_lexed_commas(&tokens[start..idx]);
            if segment.is_empty() {
                return None;
            }
            segments.push(segment);
            start = idx + 2;
            idx += 2;
            continue;
        }
        idx += 1;
    }
    if segments.is_empty() {
        return None;
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if tail.is_empty() {
        return None;
    }
    segments.push(tail);
    Some(segments)
}

fn with_each_player_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut rewritten = vec![
        OwnedLexToken::word("each", TextSpan::synthetic()),
        OwnedLexToken::word("player", TextSpan::synthetic()),
    ];
    rewritten.extend_from_slice(tokens);
    rewritten
}

fn single_each_player_effect(mut effects: Vec<EffectAst>) -> Option<EffectAst> {
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_mut_slice() else {
        return None;
    };
    (nested.len() == 1).then(|| nested.remove(0))
}

/// Preserve a per-player result set across an intervening action:
/// `Each player exiles ..., then sacrifices ..., then puts all cards they
/// exiled this way onto the battlefield.`
///
/// The first action's affected objects are tagged inside the player loop, so
/// the final move consumes only that player's exile result rather than the
/// intervening sacrifice result or unrelated cards already in exile.
pub(super) fn parse_each_player_exile_sacrifice_return_exiled(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, crate::cards::builders::CardTextError> {
    let words = parser_token_word_refs(tokens);
    if !matches!(words.as_slice(), ["each", "player", ..]) {
        return Ok(None);
    }
    let Some(segments) = comma_then_segments(tokens) else {
        return Ok(None);
    };
    let [first_tokens, second_tokens, third_tokens] = segments.as_slice() else {
        return Ok(None);
    };

    let Some(first) = single_each_player_effect(super::parse_effect_sentence_lexed(first_tokens)?)
    else {
        return Ok(None);
    };
    if !matches!(
        first,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Exile { .. } | SubjectVerbActionAst::ExileAll { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    let second_with_subject = with_each_player_subject(second_tokens);
    let Some(second) =
        single_each_player_effect(super::parse_effect_sentence_lexed(&second_with_subject)?)
    else {
        return Ok(None);
    };
    if !matches!(
        second,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Sacrifice { .. }
                | SubjectVerbActionAst::SacrificeAll { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    let third_words = parser_token_word_refs(third_tokens);
    let is_linked_return = third_words
        .first()
        .is_some_and(|word| matches!(*word, "puts" | "put"))
        && third_words.iter().any(|word| *word == "all")
        && third_words
            .iter()
            .any(|word| matches!(*word, "card" | "cards"))
        && third_words
            .windows(3)
            .any(|window| window == ["they", "exiled", "this"])
        && third_words
            .windows(2)
            .any(|window| window == ["this", "way"])
        && (third_words
            .windows(2)
            .any(|window| window == ["onto", "battlefield"])
            || third_words
                .windows(3)
                .any(|window| window == ["onto", "the", "battlefield"]));
    if !is_linked_return {
        return Ok(None);
    }

    let exiled_tag =
        crate::runtime_backend::util::helper_tag_for_tokens(first_tokens, "exiled_this_way");
    let return_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    let put_exiled = EffectAst::subject_verb_put_onto_battlefield(
        PlayerAst::That,
        TargetAst::Object(return_filter, None, None),
        false,
        ReturnControllerAst::Preserve,
    );

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::TagAffected {
                effect: Box::new(first),
                tag: exiled_tag,
            },
            second,
            put_exiled,
        ],
    }]))
}

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
            result_conjunction: false,
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
    let words = parser_token_word_refs(tokens);
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
    use crate::cards::builders::{
        EffectAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst,
    };
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
            ["Each", "opponent", "discards", "a", "card"]
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
                "Each",
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
        let effects = match effects.as_slice() {
            [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
            effects => effects,
        };
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

    #[test]
    fn each_player_exile_sacrifice_return_keeps_the_exile_result_set() {
        let tokens = lex_line(
            "Each player exiles all artifact cards from their graveyard, then sacrifices all artifacts they control, then puts all cards they exiled this way onto the battlefield.",
            0,
        )
        .expect("result-set sequence should lex");
        assert!(
            super::parse_each_player_exile_sacrifice_return_exiled(&tokens)
                .expect("specialized result-set parser should not fail")
                .is_some(),
            "specialized result-set parser should claim {tokens:#?}"
        );
        let effects =
            parse_effect_sentence_lexed(&tokens).expect("result-set sequence should parse");
        let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
            panic!("expected one each-player sequence, got {effects:#?}");
        };
        let nested = match nested.as_slice() {
            [EffectAst::CommaThen { effects }] => effects.as_slice(),
            effects => effects,
        };
        let [
            EffectAst::TagAffected { tag, .. },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Sacrifice { .. } | SubjectVerbActionAst::SacrificeAll { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PutOntoBattlefield { target, .. },
                ..
            }),
        ] = nested
        else {
            panic!("expected tagged exile, sacrifice, and return, got {nested:#?}");
        };
        let TargetAst::Object(filter, None, None) = target else {
            panic!("expected an untargeted tagged-exile filter");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *tag
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
    }
}

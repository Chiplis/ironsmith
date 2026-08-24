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
    if !crate::word_primitives::parse_sequence_prefix(&words, &["each", "player"]) {
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
    let is_linked_return = crate::word_primitives::first_is_any(&third_words, &["puts", "put"])
        && crate::slice_primitives::contains(&third_words, &"all")
        && crate::slice_primitives::contains_any(&third_words, &["card", "cards"])
        && crate::word_primitives::sequence_occurs(&third_words, &["they", "exiled", "this"])
        && crate::word_primitives::sequence_occurs(&third_words, &["this", "way"])
        && crate::word_primitives::any_sequence_occurs(
            &third_words,
            &[&["onto", "battlefield"], &["onto", "the", "battlefield"]],
        );
    if !is_linked_return {
        return Ok(None);
    }

    let exiled_tag = crate::util::helper_tag_for_tokens(first_tokens, "exiled_this_way");
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
    if !crate::word_primitives::parse_sequence_complete(
        &token_word_refs(tokens),
        &[
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
        ],
    ) {
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

    let moved_tag = crate::tag::CompilerReferenceTag::JointDiscardOrSacrifice.key();
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

/// Split a coordinated sequence when a quantified-player action is followed
/// by one or more actions with their own explicit player subjects.
///
/// The ordinary chain splitter intentionally requires a bare verb immediately
/// after a comma. That keeps noun lists intact, but it also leaves sequences
/// such as `each player mills ..., then each opponent discards ... and you
/// draw ...` as one quantified action. That incorrectly repeats the later
/// opponent/controller actions once for every player. This recognizer is
/// deliberately narrow: the first clause and every split tail must begin with
/// an explicit quantified-player or `you <effect verb>` subject. Shared-
/// subject tails such as `each player mills ..., then draws ...` remain nested
/// in the original fanout.
pub(super) fn split_explicit_player_subject_clauses(
    tokens: &[OwnedLexToken],
) -> Option<Vec<&[OwnedLexToken]>> {
    if !starts_quantified_player_action(tokens) {
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

        let boundary_start = if token.kind == TokenKind::Comma || token.is_word("and") {
            idx + 1
        } else {
            continue;
        };
        let Some((next_start, tail)) =
            explicit_player_action_after_boundary(tokens, boundary_start)
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
    if !starts_explicit_player_action(tail) {
        return None;
    }
    clauses.push(tail);
    Some(clauses)
}

fn starts_quantified_player_action(tokens: &[OwnedLexToken]) -> bool {
    let Some(shape) =
        super::super::grammar::effects::for_each_shapes::parse_participant_clause_shape(tokens)
    else {
        return false;
    };
    shape.participant_is_actor
        && find_verb_lexed(shape.inner_tokens).is_some_and(|(_, verb_idx)| verb_idx == 0)
}

fn starts_controller_action(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    crate::word_primitives::parse_sequence_prefix(&words, &["you"])
        && find_verb_lexed(tokens).is_some_and(|(_, verb_idx)| verb_idx == 1)
}

fn starts_explicit_player_action(tokens: &[OwnedLexToken]) -> bool {
    starts_controller_action(tokens) || starts_quantified_player_action(tokens)
}

fn explicit_player_action_after_boundary(
    tokens: &[OwnedLexToken],
    mut start: usize,
) -> Option<(usize, &[OwnedLexToken])> {
    let mut tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    if token_word_refs(tail)
        .first()
        .is_some_and(|word| matches!(*word, "and" | "then"))
    {
        start = start.saturating_add(1);
        tail = trim_lexed_commas(tokens.get(start..).unwrap_or_default());
    }
    starts_explicit_player_action(tail).then_some((start, tail))
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{
        EffectAst, SubjectVerbActionAst, SubjectVerbEffectAst, TargetAst,
    };
    use crate::lexer::lex_line;

    use super::super::parse_effect_sentence_lexed;
    use super::split_explicit_player_subject_clauses;

    #[test]
    fn splits_quantified_opponent_then_explicit_controller_actions() {
        let tokens = lex_line(
            "Each opponent discards a card, you draw a card, and you gain 2 life.",
            0,
        )
        .expect("player sequence should lex");
        let clauses = split_explicit_player_subject_clauses(&tokens)
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
        let clauses = split_explicit_player_subject_clauses(&tokens)
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
    fn splits_each_player_then_each_opponent_then_controller_actions() {
        let tokens = lex_line(
            "Each player mills three cards, then each opponent discards a card and you draw a card.",
            0,
        )
        .expect("quantified-player sequence should lex");
        let clauses = split_explicit_player_subject_clauses(&tokens)
            .expect("each explicit player subject should start a new scope");
        assert_eq!(clauses.len(), 3);
        assert_eq!(
            super::token_word_refs(clauses[0]),
            ["Each", "player", "mills", "three", "cards"]
        );
        assert_eq!(
            super::token_word_refs(clauses[1]),
            ["each", "opponent", "discards", "a", "card"]
        );
        assert_eq!(
            super::token_word_refs(clauses[2]),
            ["you", "draw", "a", "card"]
        );
    }

    #[test]
    fn shared_subject_tail_does_not_end_quantified_player_scope() {
        let tokens = lex_line("Each player mills three cards, then draws a card.", 0)
            .expect("shared-subject sequence should lex");
        assert!(split_explicit_player_subject_clauses(&tokens).is_none());
    }

    #[test]
    fn parses_explicit_player_subjects_as_three_top_level_actions() {
        let tokens = lex_line(
            "Each player mills three cards, then each opponent discards a card and you draw a card.",
            0,
        )
        .expect("quantified-player sequence should lex");
        let effects = parse_effect_sentence_lexed(&tokens)
            .expect("quantified-player sequence should parse structurally");
        let [EffectAst::CommaThen { effects }] = effects.as_slice() else {
            panic!("expected authored comma-then surface, got {effects:#?}");
        };
        assert!(
            matches!(
                effects.as_slice(),
                [
                    EffectAst::ForEachPlayer { effects: mill },
                    EffectAst::ForEachOpponent { effects: discard },
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Draw { .. },
                        ..
                    })
                ] if matches!(
                    mill.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Mill { .. },
                        ..
                    })]
                ) && matches!(
                    discard.as_slice(),
                    [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Discard { .. },
                        ..
                    })]
                )
            ),
            "later actions must not remain nested in the each-player scope: {effects:#?}"
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
        let [EffectAst::Coordination(coordination)] = effects.as_slice() else {
            panic!("expected canonical player-action coordination: {effects:#?}");
        };
        let canonical_effects = coordination.effects().collect::<Vec<_>>();
        assert_player_sequence(&canonical_effects);
    }

    fn assert_player_sequence(effects: &[&EffectAst]) {
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

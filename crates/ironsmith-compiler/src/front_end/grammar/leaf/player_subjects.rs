use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;
use super::references::LeafPlayerReference;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafPlayerReferenceMode {
    ControlSubject {
        allow_that_player: bool,
        allow_opponent_players: bool,
        allow_defending_player: bool,
    },
    OwnershipSubject {
        allow_opponent_players: bool,
    },
    PlayerStatusSubject,
    PlayerHasQuantitySubject,
    LifeRelationSubject,
    SpellCastThisTurnSubject,
    LifeChangeSubject,
    PlayerWouldSubject,
}

#[derive(Debug, Clone, Copy)]
struct PlayerSubjectPhrase {
    words: &'static [&'static str],
    reference: LeafPlayerReference,
}

const CONTROL_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["that", "player"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["they"], LeafPlayerReference::ThatPlayer),
    subject_phrase(
        &["defending", "player"],
        LeafPlayerReference::DefendingPlayer,
    ),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["your", "opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const OWNERSHIP_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["your", "opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const PLAYER_STATUS_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(
        &["defending", "player"],
        LeafPlayerReference::DefendingPlayer,
    ),
    subject_phrase(
        &["attacking", "player"],
        LeafPlayerReference::AttackingPlayer,
    ),
    subject_phrase(&["that", "player"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["a", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const PLAYER_HAS_QUANTITY_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["that", "player"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["they"], LeafPlayerReference::ThatPlayer),
    subject_phrase(
        &["attacking", "player"],
        LeafPlayerReference::AttackingPlayer,
    ),
    subject_phrase(
        &["defending", "player"],
        LeafPlayerReference::DefendingPlayer,
    ),
    subject_phrase(&["a", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["a", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const LIFE_RELATION_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["player", "who"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["that", "player"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["target", "opponent"], LeafPlayerReference::TargetOpponent),
    subject_phrase(&["target", "player"], LeafPlayerReference::TargetPlayer),
    subject_phrase(&["each", "opponents"], LeafPlayerReference::EachOpponent),
    subject_phrase(&["each", "opponent"], LeafPlayerReference::EachOpponent),
    subject_phrase(&["a", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["any", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["a", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(
        &["defending", "player"],
        LeafPlayerReference::DefendingPlayer,
    ),
    subject_phrase(
        &["attacking", "player"],
        LeafPlayerReference::AttackingPlayer,
    ),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const SPELL_CAST_THIS_TURN_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["that", "player"], LeafPlayerReference::ThatPlayer),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["you've"], LeafPlayerReference::You),
    subject_phrase(&["youve"], LeafPlayerReference::You),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const LIFE_CHANGE_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(
        &["one", "or", "more", "opponents"],
        LeafPlayerReference::Opponent,
    ),
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["any", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["a", "player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["player"], LeafPlayerReference::AnyPlayer),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const PLAYER_WOULD_SUBJECT_PHRASES: &[PlayerSubjectPhrase] = &[
    subject_phrase(&["an", "opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponents"], LeafPlayerReference::Opponent),
    subject_phrase(&["opponent"], LeafPlayerReference::Opponent),
    subject_phrase(&["you"], LeafPlayerReference::You),
];

const fn subject_phrase(
    words: &'static [&'static str],
    reference: LeafPlayerReference,
) -> PlayerSubjectPhrase {
    PlayerSubjectPhrase { words, reference }
}

impl LeafPlayerReferenceMode {
    fn phrases(self) -> &'static [PlayerSubjectPhrase] {
        match self {
            Self::ControlSubject { .. } => CONTROL_SUBJECT_PHRASES,
            Self::OwnershipSubject { .. } => OWNERSHIP_SUBJECT_PHRASES,
            Self::PlayerStatusSubject => PLAYER_STATUS_SUBJECT_PHRASES,
            Self::PlayerHasQuantitySubject => PLAYER_HAS_QUANTITY_SUBJECT_PHRASES,
            Self::LifeRelationSubject => LIFE_RELATION_SUBJECT_PHRASES,
            Self::SpellCastThisTurnSubject => SPELL_CAST_THIS_TURN_SUBJECT_PHRASES,
            Self::LifeChangeSubject => LIFE_CHANGE_SUBJECT_PHRASES,
            Self::PlayerWouldSubject => PLAYER_WOULD_SUBJECT_PHRASES,
        }
    }

    fn allows(self, reference: LeafPlayerReference) -> bool {
        match self {
            Self::ControlSubject {
                allow_that_player,
                allow_opponent_players,
                allow_defending_player,
            } => match reference {
                LeafPlayerReference::You => true,
                LeafPlayerReference::ThatPlayer => allow_that_player,
                LeafPlayerReference::Opponent => allow_opponent_players,
                LeafPlayerReference::DefendingPlayer => allow_defending_player,
                _ => false,
            },
            Self::OwnershipSubject {
                allow_opponent_players,
            } => match reference {
                LeafPlayerReference::You => true,
                LeafPlayerReference::Opponent => allow_opponent_players,
                _ => false,
            },
            Self::PlayerStatusSubject
            | Self::PlayerHasQuantitySubject
            | Self::LifeRelationSubject
            | Self::SpellCastThisTurnSubject
            | Self::LifeChangeSubject
            | Self::PlayerWouldSubject => true,
        }
    }
}

pub(crate) fn parse_leaf_player_reference_lexed<'a>(
    input: &mut LexStream<'a>,
    mode: LeafPlayerReferenceMode,
) -> WResult<LeafPlayerReference> {
    let checkpoint = input.clone();
    let reference = parse_player_subject_phrase_lexed(input, mode.phrases())?;
    if mode.allows(reference) {
        Ok(reference)
    } else {
        *input = checkpoint;
        Err(primitives::backtrack_err(
            "player subject",
            "player reference allowed in this production",
        ))
    }
}

pub(crate) fn parse_leaf_player_reference_tokens(
    tokens: &[OwnedLexToken],
    mode: LeafPlayerReferenceMode,
) -> Option<LeafPlayerReference> {
    let mut input = LexStream::new(tokens);
    let Ok(reference) = parse_leaf_player_reference_lexed(&mut input, mode) else {
        return None;
    };
    input.is_empty().then_some(reference)
}

pub(crate) fn parse_leaf_player_reference_words<'a>(
    words: &'a [&'a str],
    mode: LeafPlayerReferenceMode,
) -> Option<LeafPlayerReference> {
    let mut input: primitives::WordSliceInput<'a> = words;
    let Ok(reference) = parse_leaf_player_reference_word_slice(&mut input, mode) else {
        return None;
    };
    input.is_empty().then_some(reference)
}

fn parse_player_subject_phrase_lexed<'a>(
    input: &mut LexStream<'a>,
    phrases: &[PlayerSubjectPhrase],
) -> WResult<LeafPlayerReference> {
    for phrase in phrases {
        let mut probe = input.clone();
        if primitives::phrase(phrase.words)
            .parse_next(&mut probe)
            .is_ok()
        {
            *input = probe;
            return Ok(phrase.reference);
        }
    }
    Err(primitives::backtrack_err(
        "player subject",
        "recognized player-reference phrase",
    ))
}

fn parse_leaf_player_reference_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
    mode: LeafPlayerReferenceMode,
) -> WResult<LeafPlayerReference> {
    let checkpoint = *input;
    let reference = parse_player_subject_phrase_word_slice(input, mode.phrases())?;
    if mode.allows(reference) {
        Ok(reference)
    } else {
        *input = checkpoint;
        Err(primitives::backtrack_err(
            "player subject",
            "player reference allowed in this production",
        ))
    }
}

fn parse_player_subject_phrase_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
    phrases: &[PlayerSubjectPhrase],
) -> WResult<LeafPlayerReference> {
    for phrase in phrases {
        let mut probe = *input;
        if parse_word_phrase(&mut probe, phrase.words).is_ok() {
            *input = probe;
            return Ok(phrase.reference);
        }
    }
    Err(primitives::backtrack_err(
        "player subject",
        "recognized player-reference phrase",
    ))
}

fn parse_word_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    expected: &[&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(word)
            .void()
            .parse_next(input)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse_tokens(raw: &str, mode: LeafPlayerReferenceMode) -> Option<LeafPlayerReference> {
        let tokens = lex_line(raw, 0).expect("lex player-subject fixture");
        parse_leaf_player_reference_tokens(&tokens, mode)
    }

    fn assert_cases(mode: LeafPlayerReferenceMode, cases: &[(&str, LeafPlayerReference)]) {
        for (raw, expected) in cases {
            assert_eq!(parse_tokens(raw, mode), Some(*expected), "{raw}");
        }
    }

    fn control_mode() -> LeafPlayerReferenceMode {
        LeafPlayerReferenceMode::ControlSubject {
            allow_that_player: true,
            allow_opponent_players: true,
            allow_defending_player: true,
        }
    }

    #[test]
    fn existing_subject_modes_preserve_their_compatibility_languages() {
        assert_cases(
            control_mode(),
            &[
                ("you", LeafPlayerReference::You),
                ("that player", LeafPlayerReference::ThatPlayer),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
                ("your opponents", LeafPlayerReference::Opponent),
                ("defending player", LeafPlayerReference::DefendingPlayer),
            ],
        );
        assert_cases(
            LeafPlayerReferenceMode::OwnershipSubject {
                allow_opponent_players: true,
            },
            &[
                ("you", LeafPlayerReference::You),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
                ("your opponents", LeafPlayerReference::Opponent),
            ],
        );
        assert_cases(
            LeafPlayerReferenceMode::PlayerStatusSubject,
            &[
                ("you", LeafPlayerReference::You),
                ("defending player", LeafPlayerReference::DefendingPlayer),
                ("attacking player", LeafPlayerReference::AttackingPlayer),
                ("that player", LeafPlayerReference::ThatPlayer),
                ("an opponent", LeafPlayerReference::Opponent),
                ("opponent", LeafPlayerReference::Opponent),
                ("a player", LeafPlayerReference::AnyPlayer),
                ("player", LeafPlayerReference::AnyPlayer),
            ],
        );
    }

    #[test]
    fn deferred_quantity_and_life_relation_modes_preserve_odd_spellings() {
        assert_cases(
            LeafPlayerReferenceMode::PlayerHasQuantitySubject,
            &[
                ("you", LeafPlayerReference::You),
                ("a opponent", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
                ("opponent", LeafPlayerReference::Opponent),
                ("a player", LeafPlayerReference::AnyPlayer),
                ("player", LeafPlayerReference::AnyPlayer),
                ("that player", LeafPlayerReference::ThatPlayer),
                ("attacking player", LeafPlayerReference::AttackingPlayer),
                ("defending player", LeafPlayerReference::DefendingPlayer),
            ],
        );
        assert_cases(
            LeafPlayerReferenceMode::LifeRelationSubject,
            &[
                ("you", LeafPlayerReference::You),
                ("that player", LeafPlayerReference::ThatPlayer),
                ("player who", LeafPlayerReference::ThatPlayer),
                ("target player", LeafPlayerReference::TargetPlayer),
                ("target opponent", LeafPlayerReference::TargetOpponent),
                ("each opponent", LeafPlayerReference::EachOpponent),
                ("each opponents", LeafPlayerReference::EachOpponent),
                ("a opponent", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("a player", LeafPlayerReference::AnyPlayer),
                ("any player", LeafPlayerReference::AnyPlayer),
                ("player", LeafPlayerReference::AnyPlayer),
                ("defending player", LeafPlayerReference::DefendingPlayer),
                ("attacking player", LeafPlayerReference::AttackingPlayer),
            ],
        );
    }

    #[test]
    fn deferred_event_modes_preserve_exact_subject_languages() {
        assert_cases(
            LeafPlayerReferenceMode::SpellCastThisTurnSubject,
            &[
                ("that player", LeafPlayerReference::ThatPlayer),
                ("you", LeafPlayerReference::You),
                ("youve", LeafPlayerReference::You),
                ("you've", LeafPlayerReference::You),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
            ],
        );
        assert_cases(
            LeafPlayerReferenceMode::LifeChangeSubject,
            &[
                ("you", LeafPlayerReference::You),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
                ("one or more opponents", LeafPlayerReference::Opponent),
                ("a player", LeafPlayerReference::AnyPlayer),
                ("any player", LeafPlayerReference::AnyPlayer),
                ("player", LeafPlayerReference::AnyPlayer),
            ],
        );
        assert_cases(
            LeafPlayerReferenceMode::PlayerWouldSubject,
            &[
                ("you", LeafPlayerReference::You),
                ("opponent", LeafPlayerReference::Opponent),
                ("opponents", LeafPlayerReference::Opponent),
                ("an opponent", LeafPlayerReference::Opponent),
            ],
        );
    }

    #[test]
    fn production_options_and_mode_boundaries_are_enforced() {
        let restricted_control = LeafPlayerReferenceMode::ControlSubject {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
        };
        assert_eq!(
            parse_tokens("you", restricted_control),
            Some(LeafPlayerReference::You)
        );
        for raw in ["that player", "opponent", "defending player"] {
            assert_eq!(parse_tokens(raw, restricted_control), None, "{raw}");
        }
        assert_eq!(
            parse_tokens("a opponent", LeafPlayerReferenceMode::LifeRelationSubject),
            Some(LeafPlayerReference::Opponent)
        );
        assert_eq!(
            parse_tokens("a opponent", LeafPlayerReferenceMode::PlayerWouldSubject),
            None
        );
        assert_eq!(
            parse_tokens(
                "each opponents",
                LeafPlayerReferenceMode::LifeRelationSubject
            ),
            Some(LeafPlayerReference::EachOpponent)
        );
        assert_eq!(
            parse_tokens("each opponents", LeafPlayerReferenceMode::LifeChangeSubject),
            None
        );
        assert_eq!(
            parse_tokens("you've", LeafPlayerReferenceMode::SpellCastThisTurnSubject),
            Some(LeafPlayerReference::You)
        );
        assert_eq!(
            parse_tokens("you've", LeafPlayerReferenceMode::PlayerWouldSubject),
            None
        );
    }

    #[test]
    fn token_and_word_adapters_require_full_consumption() {
        assert_eq!(parse_tokens("you nearby", control_mode()), None);
        assert_eq!(
            parse_leaf_player_reference_words(&["you", "nearby"], control_mode()),
            None
        );
        assert_eq!(
            parse_leaf_player_reference_words(
                &["player", "who"],
                LeafPlayerReferenceMode::LifeRelationSubject,
            ),
            Some(LeafPlayerReference::ThatPlayer)
        );
    }
}

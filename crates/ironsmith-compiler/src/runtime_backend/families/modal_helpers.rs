#![allow(dead_code, unused_imports)]

use crate::cards::builders::IfResultPredicate;

use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom};
use super::lexer::{LexedClause, OwnedLexToken};
pub(crate) use super::util::{
    find_activation_cost_start, is_article, non_article_word_refs, replace_unbound_x_with_value,
    starts_with_activation_cost, value_contains_unbound_x,
};

const MODAL_RESULT_SUBJECT_WORDS: &[&str] = &["if", "when", "you", "they", "player", "players"];
const RESULT_VERB_WORDS: &[&str] = &[
    "remove",
    "removed",
    "sacrifice",
    "sacrificed",
    "discard",
    "discarded",
    "exile",
    "exiled",
];
const CONTRACTED_NEGATION_WORDS: &[&str] = &["dont", "doesnt", "didnt", "cant"];
const SPLIT_NEGATION_FIRST_WORDS: &[&str] = &["do", "does", "did", "can"];
const YOU_DO_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::phrase(&["you", "do"])]);
const THEY_DO_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["they", "do"])]);
const PLAYER_DOES_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::any_phrase(&[
    &["player", "do"],
    &["player", "does"],
    &["players", "do"],
    &["players", "does"],
])]);
const NO_ONE_DOES_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::any_phrase(&[
    &["no", "one", "do"],
    &["no", "one", "does"],
])]);
const YOU_WIN_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[&["you", "win"], &["you", "won"]])]);
const YOU_SEARCHED_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&["you", "searched"])]);
const PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[
        &["a", "player", "is", "dealt", "damage", "this", "way"],
        &["player", "is", "dealt", "damage", "this", "way"],
    ])]);
const SPELL_COUNTERED_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_phrase(&[&["that", "spell"], &["it", "spell"]]),
]);
const DIES_THIS_WAY_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::any_phrase(&[
    &["that", "creature", "dies", "this", "way"],
    &["that", "permanent", "dies", "this", "way"],
    &["that", "card", "dies", "this", "way"],
    &["it", "creature", "dies", "this", "way"],
    &["it", "permanent", "dies", "this", "way"],
    &["it", "card", "dies", "this", "way"],
])]);
const WOULD_DIE_THIS_TURN_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::any_phrase(&[
        &[
            "creature", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ],
        &[
            "permanent",
            "dealt",
            "damage",
            "this",
            "way",
            "would",
            "die",
            "this",
            "turn",
        ],
        &[
            "card", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ],
    ])]);
const EXCESS_DAMAGE_THIS_WAY_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::phrase(&[
        "it", "deals", "excess", "damage", "this", "way",
    ])]);
const EXCESS_DAMAGE_WAS_DEALT_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::phrase(&["excess", "damage", "was", "dealt", "to"]),
]);
const POWER_BECOMES_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::any_phrase(&[&["its", "power", "becomes"], &["it", "power", "becomes"]]),
]);

fn modal_non_article_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| token.as_word().is_some_and(|word| !is_article(word)))
        .cloned()
        .collect()
}

fn modal_clause_matches_pattern(clause: LexedClause<'_>, pattern: LexPattern<'static>) -> bool {
    pattern.matches_clause(clause)
}

fn modal_clause_matches_prefix(clause: LexedClause<'_>, pattern: LexPattern<'static>) -> bool {
    pattern.matches_prefix(clause)
}

fn modal_words_end_this_way(words: &[&str]) -> bool {
    words.ends_with(&["this", "way"])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalResultSubject {
    If,
    When,
    You,
    They,
    Player,
    Players,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalResultShape {
    ThisWay {
        subject: ModalResultSubject,
        negated: bool,
    },
    ExactNegated {
        subject: ModalResultSubject,
    },
}

fn modal_result_subject_from_clause(clause: LexedClause<'_>) -> Option<ModalResultSubject> {
    match clause.word_refs().as_slice() {
        ["if"] => Some(ModalResultSubject::If),
        ["when"] => Some(ModalResultSubject::When),
        ["you"] => Some(ModalResultSubject::You),
        ["they"] => Some(ModalResultSubject::They),
        ["player"] => Some(ModalResultSubject::Player),
        ["players"] => Some(ModalResultSubject::Players),
        _ => None,
    }
}

fn parse_modal_result_shape_from_clause(clause: LexedClause<'_>) -> Option<ModalResultShape> {
    const RESULT_QUALIFIER_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::object(
        "qualifier",
        LexCaptureKind::OneOf(&["it", "them", "that"]),
    )];
    const THIS_WAY_RESULT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS)),
        LexPattern::action("result", LexCaptureKind::OneOf(RESULT_VERB_WORDS)),
        LexPattern::optional(RESULT_QUALIFIER_ATOMS),
        LexPattern::phrase(&["this", "way"]),
    ]);
    const CONTRACTED_NEGATED_THIS_WAY_RESULT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS)),
        LexPattern::modifier("negation", LexCaptureKind::OneOf(CONTRACTED_NEGATION_WORDS)),
        LexPattern::action("result", LexCaptureKind::OneOf(RESULT_VERB_WORDS)),
        LexPattern::optional(RESULT_QUALIFIER_ATOMS),
        LexPattern::phrase(&["this", "way"]),
    ]);
    const SPLIT_NEGATED_THIS_WAY_RESULT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS)),
        LexPattern::modifier(
            "negation",
            LexCaptureKind::OneOf(SPLIT_NEGATION_FIRST_WORDS),
        ),
        LexPattern::word("not"),
        LexPattern::action("result", LexCaptureKind::OneOf(RESULT_VERB_WORDS)),
        LexPattern::optional(RESULT_QUALIFIER_ATOMS),
        LexPattern::phrase(&["this", "way"]),
    ]);
    const CONTRACTED_EXACT_NEGATED_RESULT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS)),
        LexPattern::modifier("negation", LexCaptureKind::OneOf(CONTRACTED_NEGATION_WORDS)),
    ]);
    const SPLIT_EXACT_NEGATED_RESULT_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::OneOf(MODAL_RESULT_SUBJECT_WORDS)),
        LexPattern::modifier(
            "negation",
            LexCaptureKind::OneOf(SPLIT_NEGATION_FIRST_WORDS),
        ),
        LexPattern::word("not"),
    ]);

    for pattern in [
        CONTRACTED_NEGATED_THIS_WAY_RESULT_PATTERN,
        SPLIT_NEGATED_THIS_WAY_RESULT_PATTERN,
    ] {
        if let Some(matched) = pattern.match_clause(clause) {
            let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
            matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
            matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
            return Some(ModalResultShape::ThisWay {
                subject: modal_result_subject_from_clause(subject_clause)?,
                negated: true,
            });
        }
    }

    if let Some(matched) = THIS_WAY_RESULT_PATTERN.match_clause(clause) {
        let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
        matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
        return Some(ModalResultShape::ThisWay {
            subject: modal_result_subject_from_clause(subject_clause)?,
            negated: false,
        });
    }

    for pattern in [
        CONTRACTED_EXACT_NEGATED_RESULT_PATTERN,
        SPLIT_EXACT_NEGATED_RESULT_PATTERN,
    ] {
        if let Some(matched) = pattern.match_clause(clause) {
            let subject_clause = matched.capture_clause_by_role(LexCaptureRole::Subject, clause)?;
            matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
            return Some(ModalResultShape::ExactNegated {
                subject: modal_result_subject_from_clause(subject_clause)?,
            });
        }
    }

    None
}

pub(crate) fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    let normalized_tokens = modal_non_article_word_tokens(tokens);
    let clause = LexedClause::new(&normalized_tokens);
    let words = clause.word_refs();

    if words.is_empty() {
        None
    } else {
        match parse_modal_result_shape_from_clause(clause)? {
            ModalResultShape::ThisWay {
                subject: ModalResultSubject::If | ModalResultSubject::When,
                negated: false,
            }
            | ModalResultShape::ExactNegated {
                subject: ModalResultSubject::If | ModalResultSubject::When,
            } => Some(IfResultPredicate::Did),
            ModalResultShape::ThisWay {
                subject: ModalResultSubject::If | ModalResultSubject::When,
                negated: true,
            } => Some(IfResultPredicate::DidNot),
            _ => None,
        }
    }
}

pub(crate) fn parse_if_result_predicate_lexed(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    let normalized_tokens = modal_non_article_word_tokens(tokens);
    let clause = LexedClause::new(&normalized_tokens);
    let words = clause.word_refs();
    let modal_result_shape = parse_modal_result_shape_from_clause(clause);

    if modal_clause_matches_pattern(clause, YOU_DO_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if modal_clause_matches_prefix(clause, YOU_WIN_PREFIX_PATTERN)
        && (words.len() == 2 || words.contains(&"clash"))
    {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    if modal_clause_matches_pattern(clause, THEY_DO_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if modal_clause_matches_pattern(clause, PLAYER_DOES_PATTERN) {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 6
        && modal_clause_matches_prefix(clause, YOU_SEARCHED_PREFIX_PATTERN)
        && modal_words_end_this_way(&words)
    {
        return Some(IfResultPredicate::Did);
    }
    if matches!(
        modal_result_shape,
        Some(ModalResultShape::ThisWay {
            subject: ModalResultSubject::You | ModalResultSubject::They,
            negated: false,
        })
    ) {
        return Some(IfResultPredicate::Did);
    }
    if modal_clause_matches_pattern(clause, NO_ONE_DOES_PATTERN) {
        return Some(IfResultPredicate::DidNot);
    }
    if modal_clause_matches_pattern(clause, PLAYER_DEALT_DAMAGE_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && modal_clause_matches_prefix(clause, SPELL_COUNTERED_SUBJECT_PATTERN)
        && words.contains(&"countered")
        && modal_words_end_this_way(&words)
    {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5 && modal_clause_matches_prefix(clause, DIES_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if words.len() >= 8 && modal_clause_matches_prefix(clause, WOULD_DIE_THIS_TURN_PATTERN) {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if modal_clause_matches_prefix(clause, EXCESS_DAMAGE_WAS_DEALT_PREFIX_PATTERN)
        && words.contains(&"creature")
        && modal_words_end_this_way(&words)
    {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }

    if modal_clause_matches_pattern(clause, EXCESS_DAMAGE_THIS_WAY_PATTERN) {
        return Some(IfResultPredicate::Did);
    }

    if words.len() == 5
        && modal_clause_matches_prefix(clause, POWER_BECOMES_PREFIX_PATTERN)
        && modal_words_end_this_way(&words)
    {
        return Some(IfResultPredicate::Did);
    }

    if matches!(
        modal_result_shape,
        Some(
            ModalResultShape::ThisWay {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players,
                negated: true,
            } | ModalResultShape::ExactNegated {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players,
            }
        )
    ) {
        return Some(IfResultPredicate::DidNot);
    }

    None
}

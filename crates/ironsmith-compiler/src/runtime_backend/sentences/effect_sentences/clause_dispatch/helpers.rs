use super::*;

const THE_CONTROLLER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "controller", "of"]);
const CONTROLLER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["controller", "of"]);
const THE_OWNER_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "owner", "of"]);
const OWNER_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["owner", "of"]);
const COUNTER_ON_PRONOUN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["counter", "on", "it"],
            &["counter", "on", "them"],
            &["counters", "on", "it"],
            &["counters", "on", "them"],
        ]
);
const ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERNS: &[&[&str]] = &[
    &["enchanted", "creature", "s", "controller"],
    &["enchanted", "creatures", "controller"],
    &["enchanted", "creature's", "controller"],
    &["enchanted", "creature", "s", "owner"],
    &["enchanted", "creatures", "owner"],
    &["enchanted", "creature's", "owner"],
];
const ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERNS);
const BASE_POWER_TOUGHNESS_WORDS: &[&str] = &["base", "power", "and", "toughness"];
const BASE_POWER_TOUGHNESS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & BASE_POWER_TOUGHNESS_WORDS);
const BASE_POWER_TOUGHNESS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [BASE_POWER_TOUGHNESS_WORDS]);
const WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const AND_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["and"], &["or"]]);

pub(super) fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    LexedClause::new(tokens).text()
}

fn contains_card_type(card_types: &[CardType], target: CardType) -> bool {
    crate::slice_primitives::contains(card_types, &target)
}

pub(super) fn push_unique_card_type(card_types: &mut Vec<CardType>, card_type: CardType) {
    crate::slice_primitives::push_unique(card_types, card_type);
}

pub(super) fn push_unique_subtype(subtypes: &mut Vec<Subtype>, subtype: Subtype) {
    crate::slice_primitives::push_unique(subtypes, subtype);
}

pub(super) fn parse_controller_or_owner_of_target_subject(
    subject_tokens: &[OwnedLexToken],
) -> Option<(SubjectAst, TargetAst)> {
    let subject_clause = LexedClause::new(subject_tokens);
    let subject_words = subject_clause.word_refs();
    fn strip_trailing_possessive_token(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
        let mut normalized = tokens.to_vec();
        if let Some(last) = normalized.last_mut()
            && let Some(word) = last.as_word().map(str::to_string)
        {
            let stripped = word
                .strip_suffix("'s")
                .or_else(|| word.strip_suffix("’s"))
                .or_else(|| word.strip_suffix("s'"))
                .or_else(|| word.strip_suffix("s’"));
            if let Some(stripped) = stripped {
                last.replace_word(stripped.to_string());
            }
        }
        normalized
    }

    let enchanted_filter = || {
        let mut filter = ObjectFilter::creature();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: TagKey::from("enchanted"),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        TargetAst::Object(filter, None, None)
    };
    if ENCHANTED_CREATURE_CONTROLLER_OR_OWNER_PATTERN.matches_words(&subject_words) {
        let player = if matches!(
            subject_words.last().copied(),
            Some("controller" | "controller's")
        ) {
            PlayerAst::ItsController
        } else {
            PlayerAst::ItsOwner
        };
        return Some((SubjectAst::Player(player), enchanted_filter()));
    }

    if subject_words.len() >= 2 {
        let player = match subject_words.last().copied() {
            Some("controller" | "controllers" | "controller's" | "controllers'") => {
                Some(PlayerAst::ItsController)
            }
            Some("owner" | "owners" | "owner's" | "owners'") => Some(PlayerAst::ItsOwner),
            _ => None,
        };
        if let Some(player) = player {
            let owner_word_idx = subject_words.len() - 1;
            let target_clause = subject_clause.before_word(owner_word_idx)?.trimmed();
            let target_tokens = strip_trailing_possessive_token(target_clause.tokens());
            if !target_tokens.is_empty()
                && let Ok(target) = parse_target_phrase(&target_tokens)
            {
                return Some((SubjectAst::Player(player), target));
            }
        }
    }

    let (player, target_start) = if THE_CONTROLLER_OF_PREFIX_PATTERN.matches_words(&subject_words) {
        (PlayerAst::ItsController, 3usize)
    } else if CONTROLLER_OF_PREFIX_PATTERN.matches_words(&subject_words) {
        (PlayerAst::ItsController, 2usize)
    } else if THE_OWNER_OF_PREFIX_PATTERN.matches_words(&subject_words) {
        (PlayerAst::ItsOwner, 3usize)
    } else if OWNER_OF_PREFIX_PATTERN.matches_words(&subject_words) {
        (PlayerAst::ItsOwner, 2usize)
    } else {
        return None;
    };

    let target_clause = subject_clause.from_word(target_start)?.trimmed();
    if target_clause.is_empty() {
        return None;
    }

    let target = parse_target_phrase(target_clause.tokens()).ok()?;
    Some((SubjectAst::Player(player), target))
}

pub(super) fn parse_subtype_word_or_plural(word: &str) -> Option<Subtype> {
    parse_subtype_flexible(word)
}

pub(super) fn has_counter_state_pronoun(subject_words: &[&str]) -> bool {
    subject_words
        .windows(3)
        .any(|window| COUNTER_ON_PRONOUN_PATTERN.matches_words(window))
}

pub(super) fn subject_references_base_power_toughness(subject_words: &[&str]) -> bool {
    BASE_POWER_TOUGHNESS_MARKER_PATTERN.matches_words(subject_words)
}

pub(super) fn strip_base_power_toughness_subject_tokens<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_words: &[&str],
) -> &'a [OwnedLexToken] {
    let Some(base_word_idx) = subject_words
        .windows(BASE_POWER_TOUGHNESS_WORDS.len())
        .position(|window| BASE_POWER_TOUGHNESS_PATTERN.matches_words(window))
    else {
        return subject_tokens;
    };
    let subject_clause = LexedClause::new(subject_tokens);
    let Some(base_token_idx) = subject_clause.token_index_for_word_index(base_word_idx) else {
        return subject_tokens;
    };

    let mut stripped = &subject_tokens[..base_token_idx];
    while crate::runtime_backend::lexer::token_slice_last_is(stripped, "s") {
        stripped = &stripped[..stripped.len().saturating_sub(1)];
    }
    stripped
}

pub(super) fn parse_become_base_pt_tail<'a>(
    become_words: &'a [&'a str],
) -> Result<Option<(&'a [&'a str], i32, i32)>, CardTextError> {
    let Some(with_idx) = become_words
        .iter()
        .position(|word| WITH_WORD_PATTERN.matches_words(&[*word]))
    else {
        return Ok(None);
    };
    let tail = &become_words[with_idx + 1..];
    if tail.len() != 5 || !BASE_POWER_TOUGHNESS_PATTERN.matches_words(&tail[..4]) {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(tail[4])?;
    Ok(Some((&become_words[..with_idx], power, toughness)))
}

pub(super) fn parse_become_creature_descriptor_words(
    descriptor_words: &[&str],
) -> Option<(Vec<CardType>, Vec<Subtype>, Option<crate::color::ColorSet>)> {
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut colors = crate::color::ColorSet::new();
    let mut saw_subtype = false;

    for word in descriptor_words {
        if AND_OR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = colors.union(color);
        } else if let Some(card_type) = parse_card_type(word) {
            push_unique_card_type(&mut card_types, card_type);
        } else if let Some(subtype) = parse_subtype_word_or_plural(word) {
            push_unique_subtype(&mut subtypes, subtype);
            saw_subtype = true;
        } else {
            return None;
        }
    }

    if saw_subtype && !contains_card_type(&card_types, CardType::Creature) {
        card_types.insert(0, CardType::Creature);
    }
    if card_types.is_empty() && !saw_subtype {
        return None;
    }

    Some((
        card_types,
        subtypes,
        if colors.is_empty() {
            None
        } else {
            Some(colors)
        },
    ))
}

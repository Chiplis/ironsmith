use super::*;

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
    if let Some(enchanted_phrase) = crate::runtime_backend::lexer::word_slice_matching_phrase(
        &subject_words,
        &[
            &["enchanted", "creature", "s", "controller"],
            &["enchanted", "creatures", "controller"],
            &["enchanted", "creature's", "controller"],
            &["enchanted", "creature", "s", "owner"],
            &["enchanted", "creatures", "owner"],
            &["enchanted", "creature's", "owner"],
        ],
    ) {
        let player = if matches!(
            enchanted_phrase.last().copied(),
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

    let (player, target_start) = if crate::runtime_backend::lexer::word_slice_starts_with(
        &subject_words,
        &["the", "controller", "of"],
    ) {
        (PlayerAst::ItsController, 3usize)
    } else if crate::runtime_backend::lexer::word_slice_starts_with(
        &subject_words,
        &["controller", "of"],
    ) {
        (PlayerAst::ItsController, 2usize)
    } else if crate::runtime_backend::lexer::word_slice_starts_with(
        &subject_words,
        &["the", "owner", "of"],
    ) {
        (PlayerAst::ItsOwner, 3usize)
    } else if crate::runtime_backend::lexer::word_slice_starts_with(
        &subject_words,
        &["owner", "of"],
    ) {
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
    for start in 0..subject_words.len().saturating_sub(2) {
        if matches!(subject_words[start], "counter" | "counters")
            && subject_words[start + 1] == "on"
            && matches!(subject_words[start + 2], "it" | "them")
        {
            return true;
        }
    }
    false
}

pub(super) fn subject_references_base_power_toughness(subject_words: &[&str]) -> bool {
    word_slice_contains_phrase(subject_words, &["base", "power", "and", "toughness"])
}

pub(super) fn strip_base_power_toughness_subject_tokens<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_words: &[&str],
) -> &'a [OwnedLexToken] {
    let Some(base_word_idx) =
        word_slice_find_phrase_start(subject_words, &["base", "power", "and", "toughness"])
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
) -> Result<Option<(&'a [&'a str], Value, Value)>, CardTextError> {
    let Some(with_idx) = word_slice_find_word(become_words, "with") else {
        return Ok(None);
    };
    let tail = &become_words[with_idx + 1..];
    if tail.len() != 5
        || !crate::runtime_backend::lexer::word_slice_eq(
            &tail[..4],
            &["base", "power", "and", "toughness"],
        )
    {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier_values(tail[4])?;
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
        if matches!(*word, "and" | "or") {
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

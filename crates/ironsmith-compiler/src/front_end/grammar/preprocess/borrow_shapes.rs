use super::super::{permission_shapes, primitives};
use crate::lexer::{TokenWordView, lex_line, render_token_slice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubjectPredicateSurface {
    pub(crate) subject: String,
    pub(crate) predicate: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BorrowAbilitySurface {
    pub(crate) phrase: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExiledSourceAbilityTailSurface {
    pub(crate) source_noun: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BorrowStaticSentenceSurface {
    Leading {
        condition: String,
        consequence: String,
    },
    Trailing {
        prefix: String,
        condition: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BorrowStaticConditionSurface {
    ExiledWithAbility {
        subject: String,
        tail: String,
        source_noun: Option<&'static str>,
    },
    HasAbility {
        subject: String,
    },
    InZone {
        plural: bool,
        subject: String,
        zone_tail: String,
    },
}

pub(crate) fn parse_subject_predicate_surface(sentence: &str) -> Option<SubjectPredicateSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    let mut verb_index = None;
    for verb in [
        "are", "is", "have", "has", "get", "gets", "gain", "gains", "lose", "loses", "become",
        "becomes",
    ] {
        if let Some((index, _, _)) = primitives::find_prefix(&tokens, || primitives::kw(verb)) {
            verb_index = Some(index);
            break;
        }
    }
    let verb_index = verb_index?;
    let subject = render_token_slice(tokens.get(..verb_index)?)
        .trim()
        .to_string();
    let predicate = render_token_slice(tokens.get(verb_index..)?)
        .trim()
        .to_string();
    (!subject.is_empty() && !predicate.is_empty())
        .then_some(SubjectPredicateSurface { subject, predicate })
}

pub(crate) fn parse_borrow_ability_surface(sentence: &str) -> Option<BorrowAbilitySurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    let mut best: Option<(usize, &'static str)> = None;
    for ability in BORROW_ABILITIES {
        let ability_words = ability.split_ascii_whitespace().collect::<Vec<_>>();
        for prefix in BORROW_PREFIXES {
            let mut expected = Vec::with_capacity(prefix.len() + ability_words.len());
            expected.extend_from_slice(prefix);
            expected.extend_from_slice(&ability_words);
            let Some(index) = permission_shapes::find_words(&words, &expected) else {
                continue;
            };
            match best {
                Some((best_index, best_phrase))
                    if index > best_index
                        || (index == best_index && best_phrase.len() >= ability.len()) => {}
                _ => best = Some((index, ability)),
            }
        }
    }
    best.map(|(_, phrase)| BorrowAbilitySurface { phrase })
}

pub(crate) fn parse_exiled_source_ability_tail(
    tail: &str,
) -> Option<ExiledSourceAbilityTailSurface> {
    let tokens = lex_line(tail.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    for source_noun in SOURCE_NOUNS {
        let possessive = format!("{source_noun}s");
        for preposition in ["with", "by"] {
            if permission_shapes::prefix_words(
                &words,
                &["exiled", preposition, "this", possessive.as_str()],
            ) && permission_shapes::suffix_words(&words, &["ability"])
            {
                return Some(ExiledSourceAbilityTailSurface { source_noun });
            }
        }
    }
    None
}

pub(crate) fn parse_borrow_static_sentence_surface(
    sentence: &str,
) -> Option<BorrowStaticSentenceSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens);
    let word_refs = words.word_refs();
    let leading_words = if permission_shapes::prefix_words(&word_refs, &["if"]) {
        1
    } else if permission_shapes::prefix_words(&word_refs, &["as", "long", "as"]) {
        3
    } else {
        0
    };
    if leading_words > 0 {
        let condition_start = words.token_index_after_words(leading_words)?;
        let (relative_comma, _, _) =
            primitives::find_prefix(tokens.get(condition_start..)?, || primitives::comma())?;
        let comma = condition_start + relative_comma;
        let condition = render_token_slice(tokens.get(condition_start..comma)?)
            .trim()
            .to_string();
        let consequence = render_token_slice(tokens.get(comma + 1..)?)
            .trim()
            .to_string();
        return (!condition.is_empty() && !consequence.is_empty()).then_some(
            BorrowStaticSentenceSurface::Leading {
                condition,
                consequence,
            },
        );
    }

    let marker = permission_shapes::find_words(&word_refs, &["as", "long", "as"])?;
    if marker == 0 {
        return None;
    }
    let prefix_range = words.token_span_for_words(0, marker)?;
    let condition_range = words.token_span_for_words(marker + 3, words.len())?;
    let prefix = render_token_slice(tokens.get(prefix_range)?)
        .trim()
        .to_string();
    let condition = render_token_slice(tokens.get(condition_range)?)
        .trim()
        .to_string();
    (!prefix.is_empty() && !condition.is_empty())
        .then_some(BorrowStaticSentenceSurface::Trailing { prefix, condition })
}

pub(crate) fn parse_borrow_static_condition_surface(
    condition: &str,
    ability: &str,
) -> Option<BorrowStaticConditionSurface> {
    let tokens = lex_line(condition.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens);
    let word_refs = words.word_refs();
    let ability_tokens = lex_line(ability.trim(), 0).ok()?;
    let ability_words = TokenWordView::new(&ability_tokens).word_refs();
    if ability_words.is_empty() {
        return None;
    }

    let mut with_ability = Vec::with_capacity(ability_words.len() + 1);
    with_ability.push("with");
    with_ability.extend_from_slice(&ability_words);
    if let Some(relative_with) =
        permission_shapes::find_words(word_refs.get(1..).unwrap_or_default(), &with_ability)
    {
        let with_index = relative_with + 1;
        let verb_index = with_index + with_ability.len();
        if matches!(word_refs.get(verb_index), Some(&"was" | &"were"))
            && word_refs.get(verb_index + 1) == Some(&"exiled")
            && matches!(word_refs.get(verb_index + 2), Some(&"with" | &"by"))
        {
            let subject_range = words.token_span_for_words(0, with_index)?;
            let tail_range = words.token_span_for_words(verb_index + 1, words.len())?;
            let subject = render_token_slice(tokens.get(subject_range)?)
                .trim()
                .to_string();
            let tail = render_token_slice(tokens.get(tail_range)?)
                .trim()
                .to_string();
            if !subject.is_empty() && !tail.is_empty() {
                let source_noun = parse_exiled_source_ability_tail(tail.as_str())
                    .map(|surface| surface.source_noun);
                return Some(BorrowStaticConditionSurface::ExiledWithAbility {
                    subject,
                    tail,
                    source_noun,
                });
            }
        }
    }

    if words.len() > ability_words.len() + 1 {
        let ability_start = words.len() - ability_words.len();
        let verb_index = ability_start.saturating_sub(1);
        if matches!(word_refs.get(verb_index), Some(&"has" | &"have"))
            && permission_shapes::exact_words(word_refs.get(ability_start..)?, &ability_words)
        {
            let subject_range = words.token_span_for_words(0, verb_index)?;
            let subject = render_token_slice(tokens.get(subject_range)?)
                .trim()
                .to_string();
            if !subject.is_empty() {
                return Some(BorrowStaticConditionSurface::HasAbility { subject });
            }
        }
    }

    for (phrase, plural) in [(["is", "in"], false), (["are", "in"], true)] {
        let Some(relative) =
            permission_shapes::find_words(word_refs.get(1..).unwrap_or_default(), &phrase)
        else {
            continue;
        };
        let verb_index = relative + 1;
        let subject_range = words.token_span_for_words(0, verb_index)?;
        let zone_range = words.token_span_for_words(verb_index + 2, words.len())?;
        let subject = render_token_slice(tokens.get(subject_range)?)
            .trim()
            .to_string();
        let zone_tail = render_token_slice(tokens.get(zone_range)?)
            .trim()
            .to_string();
        if !subject.is_empty() && !zone_tail.is_empty() {
            return Some(BorrowStaticConditionSurface::InZone {
                plural,
                subject,
                zone_tail,
            });
        }
    }

    None
}

const BORROW_PREFIXES: &[&[&str]] = &[
    &["gain"],
    &["gains"],
    &["has"],
    &["have"],
    &["with", "a"],
    &["with", "an"],
    &["put", "a"],
    &["put", "an"],
];

const BORROW_ABILITIES: &[&str] = &[
    "protection from any color",
    "double strike",
    "first strike",
    "indestructible",
    "deathtouch",
    "hexproof",
    "lifelink",
    "vigilance",
    "landwalk",
    "protection",
    "trample",
    "shroud",
    "shadow",
    "skulk",
    "flying",
    "menace",
    "reach",
    "haste",
    "fear",
];

const SOURCE_NOUNS: &[&str] = &[
    "creature",
    "permanent",
    "artifact",
    "enchantment",
    "equipment",
    "vehicle",
    "card",
    "spell",
    "object",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_borrowed_ability_surfaces() {
        assert_eq!(
            parse_borrow_ability_surface("Target creature gains double strike")
                .expect("ability")
                .phrase,
            "double strike"
        );
        assert!(matches!(
            parse_subject_predicate_surface("Creatures are indestructible"),
            Some(SubjectPredicateSurface { .. })
        ));
    }
}

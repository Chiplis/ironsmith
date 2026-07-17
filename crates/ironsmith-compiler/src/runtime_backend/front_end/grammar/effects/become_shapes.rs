use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, take_till};

use super::super::{leaf, permission_shapes, primitives};
use crate::cards::builders::{PlayerAst, SubjectAst, TagKey, TargetAst};
use crate::effect::Value;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenWordView};
use crate::target::{ChooseSpec, ObjectFilter};

#[path = "become_shapes/descriptors.rs"]
mod descriptors;
#[path = "become_shapes/subjects.rs"]
mod subjects;
#[path = "become_shapes/surface.rs"]
mod surface;

pub(crate) use descriptors::*;
pub(crate) use subjects::*;
pub(crate) use surface::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ControllerOwnerSubjectShape {
    pub(crate) subject: SubjectAst,
    pub(crate) target: TargetAst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BasePowerToughnessSubjectShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BecomePowerToughnessTail<'a> {
    pub(crate) descriptor_words: &'a [&'a str],
    pub(crate) power: Value,
    pub(crate) toughness: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct FilteredObjectAnimationShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) dependent_subject: bool,
    pub(crate) removes_all_abilities: bool,
    pub(crate) descriptor: BecomeCreatureDescriptor,
    pub(crate) power: Value,
    pub(crate) toughness: Value,
}

fn possessive_word_stem(input: &mut &str) -> WResult<String> {
    let stem: &str =
        take_till(1.., |character: char| matches!(character, '\'' | '’')).parse_next(input)?;
    let plural = alt((
        literal("'s").value(false),
        literal("’s").value(false),
        literal("'").value(true),
        literal("’").value(true),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    if !plural {
        return Ok(stem.to_string());
    }
    let Some(singular) = stem.strip_suffix('s') else {
        return Err(primitives::backtrack_err(
            "possessive subject",
            "plural s before apostrophe",
        ));
    };
    Ok(singular.to_string())
}

pub(crate) fn parse_possessive_subject_stem(word: &str) -> Option<String> {
    possessive_word_stem.parse(word).ok()
}

fn enchanted_target() -> TargetAst {
    let mut filter = ObjectFilter::creature();
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
    TargetAst::Object(filter, None, None)
}

fn controller_owner_suffix(input: &mut LexStream<'_>) -> WResult<PlayerAst> {
    alt((
        alt((primitives::kw("controller"), primitives::kw("controllers")))
            .value(PlayerAst::ItsController),
        alt((primitives::kw("owner"), primitives::kw("owners"))).value(PlayerAst::ItsOwner),
    ))
    .parse_next(input)
}

fn controller_owner_prefix(input: &mut LexStream<'_>) -> WResult<PlayerAst> {
    alt((
        primitives::phrase(&["the", "controller", "of"]).value(PlayerAst::ItsController),
        primitives::phrase(&["controller", "of"]).value(PlayerAst::ItsController),
        primitives::phrase(&["the", "owner", "of"]).value(PlayerAst::ItsOwner),
        primitives::phrase(&["owner", "of"]).value(PlayerAst::ItsOwner),
    ))
    .parse_next(input)
}

fn normalize_trailing_possessive(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    let Some(last) = normalized.last_mut() else {
        return normalized;
    };
    let Some(stem) = last.as_word().and_then(parse_possessive_subject_stem) else {
        return normalized;
    };
    last.replace_word(stem);
    normalized
}

fn parsed_controller_owner_shape(
    player: PlayerAst,
    target_tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    let target_tokens = normalize_trailing_possessive(target_tokens);
    let target = crate::runtime_backend::util::parse_target_phrase(&target_tokens).ok()?;
    Some(ControllerOwnerSubjectShape {
        subject: SubjectAst::Player(player),
        target,
    })
}

pub(crate) fn parse_controller_owner_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    const ENCHANTED_CONTROLLER: &[&[&str]] = &[
        &["enchanted", "creature", "s", "controller"],
        &["enchanted", "creatures", "controller"],
        &["enchanted", "creature's", "controller"],
    ];
    const ENCHANTED_OWNER: &[&[&str]] = &[
        &["enchanted", "creature", "s", "owner"],
        &["enchanted", "creatures", "owner"],
        &["enchanted", "creature's", "owner"],
    ];

    if ENCHANTED_CONTROLLER
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(PlayerAst::ItsController),
            target: enchanted_target(),
        });
    }
    if ENCHANTED_OWNER
        .iter()
        .any(|expected| permission_shapes::exact_tokens(tokens, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(PlayerAst::ItsOwner),
            target: enchanted_target(),
        });
    }

    if let Some((target_tokens, player)) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || controller_owner_suffix)
        && let Some(parsed) = parsed_controller_owner_shape(player, target_tokens)
    {
        return Some(parsed);
    }

    let (player, target_tokens) = primitives::parse_prefix(tokens, controller_owner_prefix)?;
    (!target_tokens.is_empty())
        .then(|| parsed_controller_owner_shape(player, target_tokens))
        .flatten()
}

pub(crate) fn parse_counter_state_pronoun_tokens(tokens: &[OwnedLexToken]) -> bool {
    [
        &["counter", "on", "it"][..],
        &["counter", "on", "them"],
        &["counters", "on", "it"],
        &["counters", "on", "them"],
    ]
    .iter()
    .any(|phrase| primitives::find_prefix(tokens, || primitives::phrase(phrase).void()).is_some())
}

pub(crate) fn parse_base_power_toughness_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BasePowerToughnessSubjectShape<'_>> {
    let (base_start, _, _) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["base", "power", "and", "toughness"])
    })?;
    let mut target_tokens = tokens.get(..base_start)?;
    while target_tokens.last().is_some_and(|token| token.is_word("s")) {
        target_tokens = &target_tokens[..target_tokens.len().saturating_sub(1)];
    }
    Some(BasePowerToughnessSubjectShape { target_tokens })
}

fn parse_modifier_words(words: &[&str]) -> Option<(Value, Value, usize)> {
    if let Some(first) = words.first()
        && let Ok((power, toughness)) = leaf::parse_leaf_pt_modifier_values_complete(first)
    {
        return Some((power, toughness, 1));
    }
    let (first, second) = (words.first()?, words.get(1)?);
    let joined = format!("{first}/{second}");
    let (power, toughness) = leaf::parse_leaf_pt_modifier_values_complete(&joined).ok()?;
    Some((power, toughness, 2))
}

pub(crate) fn parse_become_base_pt_words<'a>(
    words: &'a [&'a str],
) -> Option<BecomePowerToughnessTail<'a>> {
    if let Some(iterated) = parse_become_iterated_mana_value_pt_words(words) {
        return Some(iterated);
    }
    let with = permission_shapes::find_words(words, &["with"])?;
    let tail = words.get(with + 1..)?;
    const HEADS: &[&[&str]] = &[
        &["base", "power", "and", "base", "toughness"],
        &["base", "power", "and", "toughness"],
        &["power", "and", "toughness"],
    ];
    let head = HEADS
        .iter()
        .find(|head| permission_shapes::prefix_words(tail, head))?;
    let value_words = tail.get(head.len()..)?;
    if permission_shapes::prefix_words(value_words, &["each", "equal", "to"]) {
        let expression_words = value_words.get(3..)?;
        let (value, consumed) = parse_become_iterated_counter_value_words(expression_words)
            .map(|value| (value, expression_words.len()))
            .or_else(|| crate::runtime_backend::util::parse_value_expr_words(expression_words))?;
        return (consumed == expression_words.len()).then(|| BecomePowerToughnessTail {
            descriptor_words: &words[..with],
            power: value.clone(),
            toughness: value,
        });
    }
    let (power, toughness, consumed) = parse_modifier_words(value_words)?;
    (consumed == value_words.len()).then_some(BecomePowerToughnessTail {
        descriptor_words: &words[..with],
        power,
        toughness,
    })
}

fn parse_become_iterated_counter_value_words(words: &[&str]) -> Option<Value> {
    let mut index = usize::from(words.first().is_some_and(|word| *word == "the"));
    if !permission_shapes::starts_at_words(words, index, &["number", "of"]) {
        return None;
    }
    index += 2;

    let counter_offset = words
        .get(index..)?
        .iter()
        .position(|word| matches!(*word, "counter" | "counters"))?;
    if counter_offset > 2 {
        return None;
    }
    let counter_word = index + counter_offset;
    let counter_type = (counter_word > index)
        .then(|| {
            crate::runtime_backend::grammar::filters::parse_counter_type_words(
                &words[index..=counter_word],
            )
        })
        .flatten();
    let reference_words = words.get(counter_word + 1..)?;
    if ![
        &["on", "it"][..],
        &["on", "them"],
        &["on", "each", "of", "them"],
    ]
    .iter()
    .any(|expected| permission_shapes::exact_words(reference_words, expected))
    {
        return None;
    }

    Some(Value::CountersOn(
        Box::new(ChooseSpec::Iterated),
        counter_type,
    ))
}

pub(crate) fn parse_filtered_object_animation_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FilteredObjectAnimationShape<'_>> {
    let tokens = crate::runtime_backend::lexer::trim_lexed_commas(tokens);
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if words.is_empty() {
        return None;
    }

    let lose_all = words
        .windows(4)
        .position(|window| matches!(window, ["lose" | "loses", "all", "abilities", "and"]));
    let subject_word_end = lose_all.unwrap_or_else(|| words.len());
    let copula_search_start = lose_all.map_or(0, |start| start + 4);
    let mut parsed = None;
    for copula_word in copula_search_start..words.len() {
        if !matches!(
            words[copula_word],
            "is" | "are" | "become" | "becomes" | "its" | "it's" | "it’s"
        ) {
            continue;
        }
        let Some(power_toughness) = parse_become_base_pt_words(&words[copula_word + 1..]) else {
            continue;
        };
        let Some(descriptor) =
            parse_become_creature_descriptor_words(power_toughness.descriptor_words)
        else {
            continue;
        };
        if !descriptor
            .card_types
            .contains(&crate::types::CardType::Creature)
        {
            continue;
        }
        parsed = Some((copula_word, power_toughness, descriptor));
        break;
    }
    let (copula_word, power_toughness, descriptor) = parsed?;

    let dependent_subject = matches!(words[copula_word], "its" | "it's" | "it’s");
    let subject_word_end = if lose_all.is_some() {
        subject_word_end
    } else {
        copula_word
    };
    if subject_word_end == 0 && !dependent_subject {
        return None;
    }
    let subject_token_end = word_view.token_index_after_words(subject_word_end)?;

    Some(FilteredObjectAnimationShape {
        subject_tokens: &tokens[..subject_token_end],
        dependent_subject,
        removes_all_abilities: lose_all.is_some(),
        descriptor,
        power: power_toughness.power,
        toughness: power_toughness.toughness,
    })
}

pub(crate) fn parse_become_iterated_mana_value_pt_words<'a>(
    words: &'a [&'a str],
) -> Option<BecomePowerToughnessTail<'a>> {
    const HEADS: &[&[&str]] = &[
        &["base", "power", "and", "base", "toughness"],
        &["base", "power", "and", "toughness"],
        &["power", "and", "toughness"],
    ];
    const VALUE_REFS: &[&[&str]] = &[
        &["its", "mana", "value"],
        &["their", "mana", "value"],
        &["that", "permanent", "s", "mana", "value"],
        &["that", "permanents", "mana", "value"],
        &["that", "object", "s", "mana", "value"],
        &["that", "objects", "mana", "value"],
    ];

    let with = permission_shapes::find_words(words, &["with"])?;
    let tail = words.get(with + 1..)?;
    let head = HEADS
        .iter()
        .find(|head| permission_shapes::prefix_words(tail, head))?;
    let rhs = tail.get(head.len()..)?;
    if !permission_shapes::prefix_words(rhs, &["each", "equal", "to"]) {
        return None;
    }
    let value_words = rhs.get(3..)?;
    if !VALUE_REFS
        .iter()
        .any(|expected| permission_shapes::exact_words(value_words, expected))
    {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some(BecomePowerToughnessTail {
        descriptor_words: &words[..with],
        power: value.clone(),
        toughness: value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_controller_owner_and_base_pt_subjects() {
        let tokens = lex_line("the controller of target creature", 0).expect("lex");
        let shape = parse_controller_owner_subject_tokens(&tokens).expect("controller subject");
        assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsController));

        let tokens = lex_line("target creature's base power and toughness", 0).expect("lex");
        let shape = parse_base_power_toughness_subject_tokens(&tokens).expect("base pt subject");
        assert!(!shape.target_tokens.is_empty());
    }

    #[test]
    fn parses_become_pt_tails() {
        let words = [
            "red",
            "dragon",
            "with",
            "base",
            "power",
            "and",
            "toughness",
            "x/x",
        ];
        let shape = parse_become_base_pt_words(&words).expect("base pt tail");
        assert_eq!(shape.power, Value::X);
        assert_eq!(shape.toughness, Value::X);

        let words = [
            "a",
            "green",
            "and",
            "blue",
            "fractal",
            "with",
            "base",
            "power",
            "and",
            "toughness",
            "each",
            "equal",
            "to",
            "x",
            "plus",
            "1",
        ];
        let shape = parse_become_base_pt_words(&words).expect("dynamic base pt tail");
        let expected = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)));
        assert_eq!(shape.power, expected);
        assert_eq!(shape.toughness, expected);

        let words = [
            "creature",
            "with",
            "power",
            "and",
            "toughness",
            "each",
            "equal",
            "to",
            "their",
            "mana",
            "value",
        ];
        assert!(parse_become_iterated_mana_value_pt_words(&words).is_some());
    }

    #[test]
    fn parses_filtered_per_object_animation_shapes() {
        let cases = [
            (
                "Each noncreature artifact is an artifact creature with power and toughness each equal to its mana value.",
                false,
                false,
            ),
            (
                "Each planeswalker with one or more loyalty counters on it loses all abilities and is a creature with power and toughness each equal to the number of loyalty counters on it.",
                true,
                false,
            ),
            (
                "It's an artifact creature with power and toughness each equal to its mana value.",
                false,
                true,
            ),
        ];

        for (text, removes_all_abilities, dependent_subject) in cases {
            let tokens = lex_line(text, 0).expect("lex animation");
            let shape = parse_filtered_object_animation_tokens(&tokens)
                .unwrap_or_else(|| panic!("animation shape should parse: {text}"));
            assert_eq!(shape.removes_all_abilities, removes_all_abilities, "{text}");
            assert_eq!(shape.dependent_subject, dependent_subject, "{text}");
            assert!(
                shape
                    .descriptor
                    .card_types
                    .contains(&crate::types::CardType::Creature),
                "{text}: {shape:#?}"
            );
            if text.contains("loyalty") {
                assert!(
                    matches!(shape.power, Value::CountersOn(ref spec, Some(crate::CounterType::Loyalty)) if matches!(spec.as_ref(), ChooseSpec::Iterated)),
                    "{shape:#?}"
                );
            } else {
                assert!(
                    matches!(shape.power, Value::ManaValueOf(ref spec) if matches!(spec.as_ref(), ChooseSpec::Iterated)),
                    "{shape:#?}"
                );
            }
        }
    }
}

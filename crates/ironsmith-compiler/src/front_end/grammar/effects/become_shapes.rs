use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{literal, take_till};

use super::super::{leaf, permission_shapes, primitives};
use crate::cards::builders::{IT_TAG, PlayerAst, SubjectAst, TagKey, TargetAst};
use crate::effect::Value;
use crate::lexer::{LexStream, OwnedLexToken, TokenWordView};
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
    pub(crate) preserve_other_types: bool,
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
    let target_words = TokenWordView::new(&target_tokens).to_word_refs();
    let persistent_source_surface = crate::util::source_reference_surface_for_words(&target_words)
        .or_else(|| crate::util::this_source_surface_for_words(&target_words));
    let target = crate::util::parse_target_phrase(&target_tokens).ok()?;
    let target = match (persistent_source_surface, target) {
        (Some(surface), TargetAst::Source(_)) => {
            // A quoted attached-object ability is parsed in a temporary
            // name-only source context.  Span-indexed surface metadata
            // disappears when that nested parse returns, so carry the
            // authored source identity in the AST itself.
            TargetAst::Object(
                ObjectFilter::source().with_source_surface(surface),
                None,
                None,
            )
        }
        (Some(surface), TargetAst::Object(mut filter, target_span, reference_span))
            if filter.source =>
        {
            filter.source_surface = Some(surface);
            TargetAst::Object(filter, target_span, reference_span)
        }
        (_, target) => target,
    };
    Some(ControllerOwnerSubjectShape {
        subject: SubjectAst::Player(player),
        target,
    })
}

pub(crate) fn parse_controller_owner_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ControllerOwnerSubjectShape> {
    const TRIGGERING_STACK_CONTROLLER: &[&[&str]] = &[
        &["that", "spell", "or", "ability's", "controller"],
        &["that", "spell", "or", "ability", "s", "controller"],
        &["that", "spell", "or", "abilitys", "controller"],
    ];
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

    let words = TokenWordView::new(tokens).to_word_refs();
    if TRIGGERING_STACK_CONTROLLER
        .iter()
        .any(|expected| permission_shapes::exact_words(&words, expected))
    {
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::TriggeringSourceController,
            target: TargetAst::Tagged(TagKey::from("triggering_source"), None),
        });
    }

    if tokens.len() == 2 && tokens[0].is_word("its") {
        let player = if tokens[1].is_word("controller") {
            PlayerAst::ItsController
        } else if tokens[1].is_word("owner") {
            PlayerAst::ItsOwner
        } else {
            return None;
        };
        return Some(ControllerOwnerSubjectShape {
            subject: SubjectAst::Player(player),
            target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
        });
    }

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
            .or_else(|| crate::util::parse_value_expr_words(expression_words))?;
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
        .then(|| crate::grammar::filters::parse_counter_type_words(&words[index..=counter_word]))
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
    let tokens = crate::lexer::trim_lexed_commas(tokens);
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if words.is_empty() {
        return None;
    }

    let lose_all = words
        .windows(4)
        .position(|window| matches!(window, ["lose" | "loses", "all", "abilities", "and"]));
    let subject_word_end = lose_all.unwrap_or(words.len());
    let copula_search_start = lose_all.map_or(0, |start| start + 4);
    let mut parsed = None;
    for copula_word in copula_search_start..words.len() {
        if !matches!(
            words[copula_word],
            "is" | "are" | "become" | "becomes" | "its" | "it's" | "it’s"
        ) {
            continue;
        }
        let body_words = &words[copula_word + 1..];
        let parsed_body = parse_become_base_pt_words(body_words)
            .and_then(|power_toughness| {
                let descriptor =
                    parse_become_creature_descriptor_words(power_toughness.descriptor_words)?;
                Some((
                    power_toughness.power,
                    power_toughness.toughness,
                    descriptor,
                    false,
                ))
            })
            .or_else(|| {
                let body_words = body_words
                    .strip_prefix(&["a"])
                    .or_else(|| body_words.strip_prefix(&["an"]))
                    .unwrap_or(body_words);
                let (descriptor_words, preserve_other_types) =
                    strip_become_addition_tail_words(body_words);
                let leading = parse_become_leading_pt_shape(descriptor_words, &[])?;
                let descriptor = parse_become_creature_descriptor_words(
                    descriptor_words.get(leading.value_word_count..)?,
                )?;
                Some((
                    leading.power,
                    leading.toughness,
                    descriptor,
                    preserve_other_types,
                ))
            });
        let Some((power, toughness, descriptor, preserve_other_types)) = parsed_body else {
            continue;
        };
        if !descriptor
            .card_types
            .contains(&crate::types::CardType::Creature)
        {
            continue;
        }
        parsed = Some((
            copula_word,
            power,
            toughness,
            descriptor,
            preserve_other_types,
        ));
        break;
    }
    let (copula_word, power, toughness, descriptor, preserve_other_types) = parsed?;

    let dependent_subject = matches!(words[copula_word], "its" | "it's" | "it’s");
    let subject_word_end = if lose_all.is_some() {
        subject_word_end
    } else {
        copula_word
    };
    if subject_word_end == 0 && !dependent_subject {
        return None;
    }
    // A targeted subject or a leading one-shot duration ("Until end of turn,
    // target creature becomes ...") is an effect sentence, never a static
    // characteristic statement; the tolerant anthem-subject fallback would
    // otherwise swallow the prefix and mis-scope the animation to every
    // matching object on the battlefield.
    let subject_words = &words[..subject_word_end];
    if subject_words.contains(&"target") || subject_words.first() == Some(&"until") {
        return None;
    }
    let subject_token_end = word_view.token_index_after_words(subject_word_end)?;

    Some(FilteredObjectAnimationShape {
        subject_tokens: &tokens[..subject_token_end],
        dependent_subject,
        removes_all_abilities: lose_all.is_some(),
        preserve_other_types,
        descriptor,
        power,
        toughness,
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
    use crate::lexer::lex_line;

    #[test]
    fn parses_controller_owner_and_base_pt_subjects() {
        let tokens = lex_line("the controller of target creature", 0).expect("lex");
        let shape = parse_controller_owner_subject_tokens(&tokens).expect("controller subject");
        assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsController));

        let tokens = lex_line("that spell or ability's controller", 0).expect("lex");
        let shape = parse_controller_owner_subject_tokens(&tokens)
            .expect("triggering stack-object controller subject");
        assert_eq!(shape.subject, SubjectAst::TriggeringSourceController);
        assert!(matches!(
            shape.target,
            TargetAst::Tagged(ref tag, None) if tag.as_str() == "triggering_source"
        ));

        let tokens = lex_line("target creature's base power and toughness", 0).expect("lex");
        let shape = parse_base_power_toughness_subject_tokens(&tokens).expect("base pt subject");
        assert!(!shape.target_tokens.is_empty());
    }

    #[test]
    fn owner_of_target_keeps_heterogeneous_zone_union() {
        let tokens = lex_line(
            "the owner of target spell, nonland permanent, or card in a graveyard",
            0,
        )
        .expect("lex owner subject");
        let shape = parse_controller_owner_subject_tokens(&tokens).expect("owner subject");
        assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsOwner));
        let TargetAst::Object(filter, explicit_target, _) = shape.target else {
            panic!("expected object target union");
        };
        assert!(explicit_target.is_some());
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(crate::Zone::Stack)
                && branch.stack_kind == Some(crate::filter::StackObjectKind::Spell)
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(crate::Zone::Battlefield)
                && branch.excluded_card_types == [crate::CardType::Land]
        }));
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| branch.zone == Some(crate::Zone::Graveyard))
        );
    }

    #[test]
    fn named_possessive_controller_subject_persists_source_identity() {
        crate::util::with_source_reference_context("Hold for Ransom", || {
            let tokens = lex_line("Hold for Ransom's controller", 0).expect("lex named controller");
            let shape = parse_controller_owner_subject_tokens(&tokens).expect("controller subject");
            assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsController));
            let TargetAst::Object(filter, None, None) = shape.target else {
                panic!("named source must persist in an object-backed source target");
            };
            assert!(filter.source);
            assert_eq!(
                filter.source_surface,
                Some(crate::target::SourceReferenceSurface::FullName(
                    "Hold for Ransom".to_string()
                ))
            );
        });
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
            assert!(!shape.preserve_other_types, "{text}: {shape:#?}");
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

    #[test]
    fn parses_filtered_leading_pt_animation_in_addition_to_other_types() {
        let text = "Each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types.";
        let tokens = lex_line(text, 0).expect("lex animation");
        let shape = parse_filtered_object_animation_tokens(&tokens)
            .expect("leading-P/T additive animation should parse");

        assert!(shape.preserve_other_types, "{shape:#?}");
        assert_eq!(shape.power, Value::Fixed(4));
        assert_eq!(shape.toughness, Value::Fixed(4));
        assert!(
            shape
                .descriptor
                .card_types
                .contains(&crate::types::CardType::Creature),
            "{shape:#?}"
        );
        assert!(
            shape
                .descriptor
                .subtypes
                .contains(&crate::types::Subtype::Elemental),
            "{shape:#?}"
        );
    }
}

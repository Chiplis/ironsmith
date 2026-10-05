//! Base-characteristic assignments evaluate their values once on resolution.
//! Keep the destination's axes separate from the referenced object's axes.
use super::*;
use crate::lexer::TokenWordView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Axes {
    Power,
    Toughness,
    Both,
}

pub(super) fn subject(tokens: &[OwnedLexToken]) -> Option<(Axes, Vec<OwnedLexToken>)> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    for (axes, marker) in [
        (Axes::Both, &["base", "power", "and", "toughness"][..]),
        (
            Axes::Both,
            &["base", "power", "and", "base", "toughness"][..],
        ),
        (Axes::Power, &["base", "power"][..]),
        (Axes::Toughness, &["base", "toughness"][..]),
    ] {
        let Some(start) = words
            .windows(marker.len())
            .position(|window| window == marker)
        else {
            continue;
        };
        let after = start + marker.len();
        let (range, possessive) =
            if start <= 1 && words.get(after) == Some(&"of") && (start == 0 || words[0] == "the") {
                (view.token_span_for_words(after + 1, words.len())?, false)
            } else if start > 0 && after == words.len() {
                (view.token_span_for_words(0, start)?, true)
            } else {
                continue;
            };
        let mut target = tokens[range].to_vec();
        if possessive {
            normalize_possessive(&mut target);
        }
        if !target.is_empty() {
            return Some((axes, target));
        }
    }
    None
}

fn normalize_possessive(tokens: &mut Vec<OwnedLexToken>) {
    while tokens.last().is_some_and(|token| token.is_word("s")) {
        tokens.pop();
    }
    if let Some(last) = tokens.last_mut() {
        if let Some(stem) = last
            .as_word()
            .and_then(become_grammar::parse_possessive_subject_stem)
            .or_else(|| become_grammar::parse_possessive_subject_stem(last.literal_surface()))
        {
            last.replace_word(stem);
        } else {
            // Source-name normalization can already have folded the possessive
            // into its parser word. The surrounding characteristic suffix is
            // the proof that this is a possessive, not a plural destination.
            for (possessive, singular) in [
                ("creatures", "creature"),
                ("permanents", "permanent"),
                ("cards", "card"),
            ] {
                if last.is_word(possessive) {
                    last.replace_word(singular);
                    break;
                }
            }
        }
    }
}

struct Quantity {
    power: Value,
    toughness: Option<Value>,
    declared_reference: Option<TargetAst>,
}

fn complete_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let (value, used) = parse_value(tokens)?;
    (used == tokens.len()).then_some(value)
}

fn quantity(tokens: &[OwnedLexToken]) -> Option<Quantity> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    // Pair references share one antecedent: "that creature's power and
    // toughness" is not two unrelated scalar expressions or their sum.
    if let Some((power, toughness)) =
        crate::grammar::shared_util::value_expr::parse_power_toughness_value_pair_words(&words)
    {
        return Some(Quantity {
            power,
            toughness: Some(toughness),
            declared_reference: None,
        });
    }
    if let Some(value) = complete_value(tokens) {
        return Some(Quantity {
            power: value,
            toughness: None,
            declared_reference: None,
        });
    }
    // A characteristic of an explicitly announced target must declare that
    // target even though the permanent receiving the base-stat effect may be
    // the source. The ordinary reference pass assigns its exact object tag.
    let (fixed_prefix, rest) = if tokens.get(1).is_some_and(|token| token.is_word("plus")) {
        let (value, used) = crate::util::parse_number(tokens)?;
        if used != 1 {
            return None;
        }
        (Some(i32::try_from(value).ok()?), &tokens[2..])
    } else {
        (None, tokens)
    };
    let view = TokenWordView::new(rest);
    let words = view.word_refs();
    let (axes, reference_range, possessive) =
        if words.starts_with(&["the", "power", "and", "toughness", "of"]) {
            (
                Axes::Both,
                view.token_span_for_words(5, words.len())?,
                false,
            )
        } else if words.starts_with(&["the", "power", "of"]) {
            (
                Axes::Power,
                view.token_span_for_words(3, words.len())?,
                false,
            )
        } else if words.starts_with(&["the", "toughness", "of"]) {
            (
                Axes::Toughness,
                view.token_span_for_words(3, words.len())?,
                false,
            )
        } else if words.ends_with(&["power", "and", "toughness"]) {
            (
                Axes::Both,
                view.token_span_for_words(0, words.len() - 3)?,
                true,
            )
        } else if words.last() == Some(&"power") {
            (
                Axes::Power,
                view.token_span_for_words(0, words.len() - 1)?,
                true,
            )
        } else if words.last() == Some(&"toughness") {
            (
                Axes::Toughness,
                view.token_span_for_words(0, words.len() - 1)?,
                true,
            )
        } else {
            return None;
        };
    let mut reference = rest[reference_range].to_vec();
    if !reference
        .first()
        .is_some_and(|token| token.is_word("target"))
    {
        return None;
    }
    if possessive {
        normalize_possessive(&mut reference);
    }
    let target = parse_target_phrase(&reference).ok()?;
    let spec = Box::new(ChooseSpec::Tagged(
        crate::tag::CompilerReferenceTag::It.key(),
    ));
    let scalar = if axes == Axes::Toughness {
        Value::ToughnessOf(spec.clone())
    } else {
        Value::PowerOf(spec.clone())
    };
    let power = fixed_prefix.map_or_else(
        || scalar.clone(),
        |fixed| Value::Add(Box::new(Value::Fixed(fixed)), Box::new(scalar.clone())),
    );
    if fixed_prefix.is_some() && axes == Axes::Both {
        return None;
    }
    Some(Quantity {
        power,
        toughness: (axes == Axes::Both).then(|| Value::ToughnessOf(spec)),
        declared_reference: Some(target),
    })
}

pub(super) fn assignment(
    axes: Axes,
    target: TargetAst,
    tokens: &[OwnedLexToken],
    duration: Until,
    surface: Option<ironsmith_core::SetQuantifierSurface>,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("equal"))
        || !tokens.get(1).is_some_and(|token| token.is_word("to"))
    {
        return Ok(None);
    }
    let rhs = &tokens[2..];
    let Some(quantity) = quantity(rhs) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported base-characteristic quantity '{}'",
            render_lower_words(rhs)
        )));
    };
    if axes != Axes::Both && quantity.toughness.is_some() {
        return Err(CardTextError::ParseError(
            "a single base characteristic cannot take a power/toughness pair".into(),
        ));
    }
    let effect = match axes {
        Axes::Power => EffectAst::subject_verb_set_base_power(quantity.power, target, duration),
        Axes::Toughness => {
            EffectAst::subject_verb_set_base_toughness(quantity.power, target, duration)
        }
        Axes::Both => EffectAst::subject_verb_set_base_power_toughness(
            quantity.power.clone(),
            quantity.toughness.unwrap_or(quantity.power),
            target,
            duration,
        )
        .with_set_quantifier_surface(surface),
    };
    Ok(Some(if let Some(reference) = quantity.declared_reference {
        EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_explicit_target_only(reference),
                effect,
            ],
        }
    } else {
        effect
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{CharacteristicActionAst, SubjectVerbActionAst};
    fn lex(text: &str) -> Vec<OwnedLexToken> {
        crate::lexer::lex_line(text, 0).unwrap()
    }
    #[test]
    fn characteristic_subjects_keep_the_axis_and_strip_only_the_possessive() {
        for (text, axes, expected) in [
            ("this creature's base power", Axes::Power, "this creature"),
            (
                "this creature's base toughness",
                Axes::Toughness,
                "this creature",
            ),
            (
                "this creature's base power and toughness",
                Axes::Both,
                "this creature",
            ),
            (
                "the base power and toughness of target Human you control",
                Axes::Both,
                "target human you control",
            ),
            (
                "the base power and toughness of each other creature you control",
                Axes::Both,
                "each other creature you control",
            ),
        ] {
            let (actual, target) = subject(&lex(text)).unwrap();
            assert_eq!(actual, axes);
            assert_eq!(
                crate::lexer::parser_token_word_refs(&target).join(" "),
                expected
            );
        }
        assert!(subject(&lex("the power of target creature")).is_none());
        assert!(subject(&lex("this creature's base power plus toughness")).is_none());
    }
    #[test]
    fn referenced_pairs_are_separate_values_while_a_single_rhs_sets_both_axes_equally() {
        let paired = quantity(&lex("that creature's power and toughness")).unwrap();
        assert!(
            matches!(paired.power.unhinted(), Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(_)))
        );
        assert!(
            matches!(paired.toughness.unwrap().unhinted(), Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(_)))
        );
        let effect = assignment(
            Axes::Both,
            TargetAst::Source(None),
            &lex("equal to this creature's power"),
            Until::EndOfTurn,
            None,
        )
        .unwrap()
        .unwrap();
        let EffectAst::SubjectVerb(subject) = effect else {
            panic!("assignment");
        };
        assert!(
            matches!(subject.action, SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { ref power, ref toughness, .. }) if power == toughness)
        );
        assert!(
            assignment(
                Axes::Power,
                TargetAst::Source(None),
                &lex("equal to that creature's power and toughness"),
                Until::EndOfTurn,
                None
            )
            .is_err()
        );
    }
    #[test]
    fn dynamic_source_axis_and_separate_numeric_target_do_not_become_an_animation() {
        for (subject_text, rhs, expected) in [
            (
                "this creature's base power",
                "equal to that creature's power until end of turn",
                "SetBasePower",
            ),
            (
                "this creature's base toughness",
                "equal to 1 plus the number of creature cards in your graveyard",
                "SetBaseToughness",
            ),
            (
                "this creature's base power",
                "equal to target creature's power",
                "TargetOnly",
            ),
            (
                "this creature's base toughness",
                "equal to 1 plus the power of target creature blocking or blocked by this creature",
                "TargetOnly",
            ),
        ] {
            let (result, loss) = crate::parse_loss::capture(|| {
                super::super::parse_become_clause(&lex(subject_text), &lex(rhs))
            });
            let effect = result.unwrap_or_else(|error| panic!("{subject_text}: {error}"));
            assert!(!loss.is_lossy(), "{}", loss.reasons_text());
            let debug = format!("{effect:?}");
            assert!(
                debug.contains(expected) || expected == "TargetOnly" && debug.contains("Target("),
                "{debug}"
            );
            assert!(!debug.contains("BecomeBasePtCreature"), "{debug}");
        }
    }
    #[test]
    fn an_unknown_operand_or_duration_is_not_truncated_to_a_valid_prefix() {
        for rhs in [
            "equal to that creature's power plus mystery",
            "equal to its number of stickers",
            "equal to 4 until the end of your next upkeep",
        ] {
            assert!(
                super::super::parse_become_clause(&lex("this creature's base power"), &lex(rhs))
                    .is_err(),
                "{rhs}"
            );
        }
    }
}

/// `a Treefolk creature with haste and base power and toughness equal to ...`.
/// The pre-P/T abilities belong to the same animation and duration.
pub(super) fn animation_with_preceding_grants(
    target: TargetAst,
    tokens: &[OwnedLexToken],
    duration: Until,
    duration_surface: Option<ironsmith_core::AnimationDurationSurface>,
    set_surface: Option<ironsmith_core::SetQuantifierSurface>,
    preserve_other_types: bool,
    preserve_other_colors: bool,
) -> Result<Option<EffectAst>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let Some(and_base) = words
        .windows(5)
        .position(|window| window == ["and", "base", "power", "and", "toughness"])
    else {
        return Ok(None);
    };
    let Some(with) = words[..and_base].iter().position(|word| *word == "with") else {
        return Ok(None);
    };
    let Some(grant_range) = view.token_span_for_words(with + 1, and_base) else {
        return Ok(None);
    };
    // A quoted granted ability can itself mention base characteristics. Its
    // words must not be mistaken for the outer animation's characteristic tail.
    if tokens[..grant_range.end]
        .iter()
        .filter(|token| token.kind == TokenKind::Quote)
        .count()
        % 2
        != 0
    {
        return Ok(None);
    }
    let Some(descriptor) = become_grammar::parse_become_creature_descriptor_words(&words[..with])
    else {
        return Ok(None);
    };
    let mut pt_words = vec!["creature", "with"];
    pt_words.extend_from_slice(&words[and_base + 1..]);
    let Some(pt) = become_grammar::parse_become_base_pt_words(&pt_words) else {
        return Err(CardTextError::ParseError(
            "unsupported animation base-characteristic tail".into(),
        ));
    };
    let grants_tokens = &tokens[grant_range];
    let (granted_abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(grants_tokens, &words, false)?;
    if is_choice || granted_abilities.is_empty() {
        return Err(CardTextError::ParseError(
            "unsupported animation pre-characteristic grants".into(),
        ));
    }
    Ok(Some(
        EffectAst::subject_verb_become_base_pt_creature(
            pt.power,
            pt.toughness,
            target,
            descriptor.card_types,
            descriptor.subtypes,
            Vec::new(),
            descriptor.colors,
            Vec::new(),
            granted_abilities,
            preserve_other_types,
            preserve_other_types
                .then_some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes),
            Some(ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness),
            duration_surface,
            duration,
        )
        .with_set_quantifier_surface(set_surface)
        .with_animation_color_retention(preserve_other_colors),
    ))
}

#[cfg(test)]
mod pre_pt_grant_tests {
    use super::*;
    #[test]
    fn paired_animation_keeps_preceding_haste_and_complete_still_land_followup() {
        let tokens = crate::lexer::lex_line("Untap target land you control. It becomes a Treefolk creature with haste and base power and toughness equal to this creature's power and toughness. It's still a land.", 0).unwrap();
        let (result, loss) = crate::parse_loss::capture(|| {
            crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
        });
        let effects = result.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let debug = format!("{effects:?}");
        for required in [
            "PowerOf",
            "ToughnessOf",
            "Haste",
            "Treefolk",
            "preserve_other_types: true",
        ] {
            assert!(debug.contains(required), "{required}: {debug}");
        }
    }
    #[test]
    fn pre_pt_unknown_grants_do_not_disappear_and_unknown_quantity_tails_do_not_truncate() {
        let subject = crate::lexer::lex_line("it", 0).unwrap();
        for body in [
            "a Treefolk creature with mystery and base power and toughness equal to this creature's power and toughness",
            "a Treefolk creature with haste and base power and toughness equal to this creature's power and toughness plus mystery",
        ] {
            let tokens = crate::lexer::lex_line(body, 0).unwrap();
            assert!(super::super::parse_become_clause(&subject, &tokens).is_err());
        }
    }
}

use winnow::combinator::{alt, opt};
use winnow::prelude::*;

use crate::ConditionExpr;
use crate::cards::builders::ControlDurationAst;
use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::{filters, permission_shapes, primitives};
use crate::runtime_backend::front_end::lexer::{
    OwnedLexToken, TokenWordView, lex_line, trim_lexed_commas,
};
use crate::runtime_backend::front_end::shared::util::{
    current_source_reference_name, source_reference_surface_for_words,
    this_source_surface_for_words,
};
use crate::target::SourceReferenceSurface;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GainControlClauseShape<'a> {
    pub(crate) target_tokens: &'a [OwnedLexToken],
    pub(crate) duration_tokens: &'a [OwnedLexToken],
    pub(crate) delayed_until_end_of_combat: bool,
    pub(crate) dynamic_power_bound: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PermanentControlDurationShape {
    pub(crate) until: Until,
    pub(crate) condition: Option<ConditionExpr>,
    pub(crate) source_surface: Option<SourceReferenceSurface>,
}

fn min_offset(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn duration_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let during =
        primitives::find_prefix(tokens, || primitives::kw("during")).map(|(index, _, _)| index);
    let until =
        primitives::find_prefix(tokens, || primitives::kw("until")).map(|(index, _, _)| index);
    let conditional = primitives::find_prefix(tokens, || {
        primitives::phrase(&["for", "as", "long", "as"]).void()
    })
    .map(|(index, _, _)| index);
    min_offset(min_offset(during, until), conditional)
}

fn delayed_combat_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let without_article = primitives::find_prefix(tokens, || {
        primitives::phrase(&["at", "end", "of", "combat"]).void()
    })
    .map(|(index, _, _)| index);
    let with_article = primitives::find_prefix(tokens, || {
        primitives::phrase(&["at", "the", "end", "of", "combat"]).void()
    })
    .map(|(index, _, _)| index);
    min_offset(without_article, with_article)
}

pub(crate) fn parse_gain_control_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<GainControlClauseShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (_, after_control) = primitives::parse_prefix(tokens, primitives::kw("control").void())?;
    let after_control = primitives::parse_prefix(after_control, opt(primitives::kw("of")).void())
        .map(|(_, rest)| rest)
        .unwrap_or(after_control);
    let delayed = delayed_combat_start(after_control);
    let duration = duration_start(after_control);
    let target_len = min_offset(delayed, duration).unwrap_or(after_control.len());
    let target_tokens = trim_lexed_commas(after_control.get(..target_len)?);
    let duration_tokens = duration
        .and_then(|index| after_control.get(index..))
        .map(trim_lexed_commas)
        .unwrap_or_default();
    Some(GainControlClauseShape {
        target_tokens,
        duration_tokens,
        delayed_until_end_of_combat: delayed.is_some(),
        dynamic_power_bound: primitives::contains_word(tokens, "power")
            && primitives::contains_word(tokens, "number")
            && permission_shapes::contains_tokens(tokens, &["you", "control"]),
    })
}

fn source_surface(tokens: &[OwnedLexToken]) -> Option<SourceReferenceSurface> {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    source_reference_surface_for_words(&words)
        .or_else(|| this_source_surface_for_words(&words))
        .or_else(|| {
            let source_name = current_source_reference_name()?;
            source_name.split("//").find_map(|face_name| {
                let face_name = face_name.trim();
                let name_tokens = lex_line(face_name, 0).ok()?;
                let name_words = TokenWordView::new(&name_tokens).word_refs();
                (words.len() == name_words.len()
                    && words
                        .iter()
                        .zip(name_words)
                        .all(|(actual, expected)| actual.eq_ignore_ascii_case(expected)))
                .then(|| SourceReferenceSurface::FullName(face_name.to_string()))
            })
        })
}

fn after_you_control(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, rest) =
        primitives::find_prefix(tokens, || primitives::phrase(&["you", "control"]).void())?;
    Some(trim_lexed_commas(rest))
}

fn is_source_reference(tokens: &[OwnedLexToken]) -> bool {
    source_surface(tokens).is_some()
}

fn parses_you_control_source(tokens: &[OwnedLexToken]) -> bool {
    let Some(after_control) = after_you_control(tokens) else {
        return false;
    };
    let source_end = primitives::find_prefix(after_control, || primitives::kw("and"))
        .map(|(index, _, _)| index)
        .unwrap_or(after_control.len());
    after_control
        .get(..source_end)
        .is_some_and(is_source_reference)
        || (["this", "thiss", "source", "creature", "permanent", "saga"])
            .into_iter()
            .any(|word| primitives::contains_word(tokens, word))
}

fn parse_source_remains_tapped(tokens: &[OwnedLexToken]) -> Option<Option<SourceReferenceSurface>> {
    let after_control = after_you_control(tokens)?;
    let (and_index, _, after_and) =
        primitives::find_prefix(after_control, || primitives::kw("and"))?;
    let first_source = trim_lexed_commas(after_control.get(..and_index)?);
    if first_source.is_empty() {
        return None;
    }
    let first_surface = source_surface(first_source);
    let (remains_index, _, after_remains) = primitives::find_prefix(after_and, || {
        alt((primitives::kw("remain"), primitives::kw("remains"))).void()
    })?;
    let second_source = trim_lexed_commas(after_and.get(..remains_index)?);
    if second_source.is_empty() || !primitives::contains_word(after_remains, "tapped") {
        return None;
    }
    let second_surface = source_surface(second_source);
    let repeated_surface = TokenWordView::new(first_source).word_refs()
        == TokenWordView::new(second_source).word_refs();
    if first_surface.is_none() && second_surface.is_none() && !repeated_surface {
        return None;
    }
    Some(first_surface.or(second_surface))
}

fn has_all_words(tokens: &[OwnedLexToken], words: &[&'static str]) -> bool {
    words
        .iter()
        .all(|word| primitives::contains_word(tokens, word))
}

fn counter_duration_type(tokens: &[OwnedLexToken]) -> Option<crate::object::CounterType> {
    let (has_index, _, after_has) = primitives::find_prefix(tokens, || primitives::kw("has"))?;
    let _ = has_index;
    let (counter_index, _, _) = primitives::find_prefix(after_has, || {
        alt((primitives::kw("counter"), primitives::kw("counters"))).void()
    })?;
    let mut counter_tokens = trim_lexed_commas(after_has.get(..counter_index)?);
    if let Some((_, rest)) = primitives::parse_prefix(
        counter_tokens,
        opt(alt((primitives::kw("a"), primitives::kw("an")))).void(),
    ) {
        counter_tokens = rest;
    }
    filters::parse_counter_type_from_tokens(counter_tokens)
}

fn parse_predicate_control_duration(tokens: &[OwnedLexToken]) -> Option<Until> {
    use ironsmith_core::{
        ContinuousDurationObject as ObjectRef, ContinuousDurationPlayer as PlayerRef,
        ContinuousDurationPredicate as Predicate,
    };

    if !permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"]) {
        return None;
    }

    if has_all_words(tokens, &["power", "less", "equal", "tapped"]) {
        return Some(Until::ForAsLongAs(Predicate::all([
            Predicate::ObjectTapped(ObjectRef::Source),
            Predicate::ObjectPowerAtMostObject {
                lesser: ObjectRef::AffectedObject,
                greater: ObjectRef::Source,
            },
        ])));
    }
    if has_all_words(tokens, &["aura", "attached", "to"]) {
        return Some(Until::ForAsLongAs(Predicate::ObjectAttachedTo {
            attachment: ObjectRef::Tagged(crate::tag::TagKey::from("triggering")),
            attached_to: ObjectRef::AffectedObject,
        }));
    }
    if primitives::contains_word(tokens, "enchanted") {
        return Some(Until::ForAsLongAs(Predicate::ObjectIsEnchanted(
            ObjectRef::AffectedObject,
        )));
    }
    if primitives::contains_word(tokens, "monarch") {
        return Some(Until::ForAsLongAs(Predicate::PlayerIsMonarch(
            PlayerRef::ControllerOf(ObjectRef::AffectedObject),
        )));
    }
    if let Some(counter_type) = counter_duration_type(tokens) {
        return Some(Until::ForAsLongAs(Predicate::affected_object_has_counter(
            counter_type,
        )));
    }
    if primitives::contains_word(tokens, "tapped") {
        return Some(Until::ForAsLongAs(Predicate::ObjectTapped(
            ObjectRef::Source,
        )));
    }
    None
}

pub(crate) fn parse_control_duration_shape(tokens: &[OwnedLexToken]) -> Option<ControlDurationAst> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return Some(ControlDurationAst::Forever);
    }
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && parses_you_control_source(tokens)
    {
        return Some(ControlDurationAst::AsLongAsYouControlSource);
    }
    if has_all_words(tokens, &["during", "next", "turn"]) {
        return Some(ControlDurationAst::DuringNextTurn);
    }
    if has_all_words(tokens, &["until", "end", "next", "turn"]) {
        return Some(ControlDurationAst::UntilYourNextTurnEnd);
    }
    if has_all_words(tokens, &["until", "end", "turn"]) {
        return Some(ControlDurationAst::UntilEndOfTurn);
    }
    None
}

pub(crate) fn parse_permanent_control_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<PermanentControlDurationShape> {
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && let Some(surface) = parse_source_remains_tapped(tokens)
    {
        return Some(PermanentControlDurationShape {
            until: Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::all([
                ironsmith_core::ContinuousDurationPredicate::ObjectControlledBy {
                    object: ironsmith_core::ContinuousDurationObject::Source,
                    player: ironsmith_core::ContinuousDurationPlayer::EffectController,
                },
                ironsmith_core::ContinuousDurationPredicate::ObjectTapped(
                    ironsmith_core::ContinuousDurationObject::Source,
                ),
            ])),
            condition: None,
            source_surface: surface,
        });
    }
    if let Some(until) = parse_predicate_control_duration(tokens) {
        return Some(PermanentControlDurationShape {
            until,
            condition: None,
            source_surface: None,
        });
    }
    let duration = parse_control_duration_shape(tokens)?;
    let until = match duration {
        ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
        ControlDurationAst::UntilYourNextTurnEnd => Until::YourNextTurnEnd,
        ControlDurationAst::Forever => Until::Forever,
        ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
        ControlDurationAst::DuringNextTurn => return None,
    };
    Some(PermanentControlDurationShape {
        until,
        condition: None,
        source_surface: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn splits_control_target_duration_and_delay() {
        let tokens = lex_line(
            "control of target creature until end of turn at the end of combat",
            0,
        )
        .unwrap();
        let shape = parse_gain_control_clause_shape(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(shape.target_tokens).to_word_refs(),
            vec!["target", "creature"]
        );
        assert!(shape.delayed_until_end_of_combat);
        assert_eq!(
            parse_control_duration_shape(shape.duration_tokens),
            Some(ControlDurationAst::UntilEndOfTurn)
        );

        let tapped = lex_line(
            "for as long as you control this creature and this creature remains tapped",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_permanent_control_duration_shape(&tapped)
                .unwrap()
                .until,
            Until::ForAsLongAs(ironsmith_core::ContinuousDurationPredicate::all([
                ironsmith_core::ContinuousDurationPredicate::ObjectControlledBy {
                    object: ironsmith_core::ContinuousDurationObject::Source,
                    player: ironsmith_core::ContinuousDurationPlayer::EffectController,
                },
                ironsmith_core::ContinuousDurationPredicate::ObjectTapped(
                    ironsmith_core::ContinuousDurationObject::Source,
                ),
            ]))
        );
    }

    #[test]
    fn parses_named_source_compound_control_duration_with_surface() {
        crate::runtime_backend::front_end::shared::util::with_source_reference_context(
            "Rubinia Soulsinger",
            || {
                let tokens = lex_line(
                    "for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped",
                    0,
                )
                .unwrap();
                let shape = parse_permanent_control_duration_shape(&tokens).unwrap();
                assert!(matches!(shape.until, Until::ForAsLongAs(_)));
                assert_eq!(
                    shape.source_surface,
                    Some(SourceReferenceSurface::FullName(
                        "Rubinia Soulsinger".to_string()
                    ))
                );
            },
        );
    }

    #[test]
    fn parses_typed_latched_control_duration_predicates() {
        use ironsmith_core::{
            ContinuousDurationObject as ObjectRef, ContinuousDurationPlayer as PlayerRef,
            ContinuousDurationPredicate as Predicate,
        };

        let parse = |text| {
            let tokens = lex_line(text, 0).unwrap();
            parse_permanent_control_duration_shape(&tokens)
                .expect("predicate-bearing duration should parse")
                .until
        };
        assert_eq!(
            parse("for as long as it has a shield counter on it"),
            Until::ForAsLongAs(Predicate::affected_object_has_counter(
                crate::object::CounterType::Shield,
            ))
        );
        assert_eq!(
            parse("for as long as that creature is enchanted"),
            Until::ForAsLongAs(Predicate::ObjectIsEnchanted(ObjectRef::AffectedObject,))
        );
        assert_eq!(
            parse("for as long as they're the monarch"),
            Until::ForAsLongAs(Predicate::PlayerIsMonarch(PlayerRef::ControllerOf(
                ObjectRef::AffectedObject
            ),))
        );
        assert_eq!(
            parse("for as long as that Aura is attached to it"),
            Until::ForAsLongAs(Predicate::ObjectAttachedTo {
                attachment: ObjectRef::Tagged(crate::tag::TagKey::from("triggering")),
                attached_to: ObjectRef::AffectedObject,
            })
        );
        assert_eq!(
            parse(
                "for as long as this creature remains tapped and that creature's power remains less than or equal to this creature's power",
            ),
            Until::ForAsLongAs(Predicate::all([
                Predicate::ObjectTapped(ObjectRef::Source),
                Predicate::ObjectPowerAtMostObject {
                    lesser: ObjectRef::AffectedObject,
                    greater: ObjectRef::Source,
                },
            ]))
        );
    }
}

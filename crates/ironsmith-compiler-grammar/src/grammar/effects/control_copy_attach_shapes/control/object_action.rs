use super::*;

pub(super) fn parse_predicate_control_duration(tokens: &[OwnedLexToken]) -> Option<Until> {
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
            attachment: ObjectRef::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind()),
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

pub fn parse_control_duration_shape(tokens: &[OwnedLexToken]) -> Option<ControlDurationAst> {
    parse_control_duration_shape_with_optional_context(None, tokens)
}

pub(super) fn parse_control_duration_shape_with_optional_context(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<ControlDurationAst> {
    let tokens = trim_lexed_commas(tokens);
    if tokens.is_empty() {
        return Some(ControlDurationAst::Forever);
    }
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && parses_you_control_source(context, tokens)
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

pub fn parse_permanent_control_duration_shape(
    tokens: &[OwnedLexToken],
) -> Option<PermanentControlDurationShape> {
    parse_permanent_control_duration_shape_with_optional_context(None, tokens)
}

pub fn parse_permanent_control_duration_shape_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Option<PermanentControlDurationShape> {
    parse_permanent_control_duration_shape_with_optional_context(Some(context), tokens)
}

pub(super) fn parse_permanent_control_duration_shape_with_optional_context(
    context: Option<crate::parse_context::ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
) -> Option<PermanentControlDurationShape> {
    if permission_shapes::contains_tokens(tokens, &["for", "as", "long", "as"])
        && let Some(surface) = parse_source_remains_tapped(context, tokens)
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
    let duration = parse_control_duration_shape_with_optional_context(context, tokens)?;
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

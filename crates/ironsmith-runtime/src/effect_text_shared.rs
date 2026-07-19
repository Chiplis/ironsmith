use crate::continuous::EffectTarget;
use crate::effects::ApplyContinuousEffect;
use crate::filter::ObjectFilter;
use crate::target::ChooseSpec;

/// Stable internal attribution carried by authored target choices. These tags
/// distinguish two targets chosen by different players after both choices have
/// been made, so later clauses can still say which player chose which object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetChoiceAttribution {
    AbilityController,
    Opponent,
}

pub const ABILITY_CONTROLLER_TARGET_CHOICE_TAG: &str = "__ability_controller_target_choice_0";
pub const OPPONENT_TARGET_CHOICE_TAG: &str = "__opponent_target_choice_1";

pub fn target_choice_attribution(tag: &str) -> Option<TargetChoiceAttribution> {
    match tag {
        ABILITY_CONTROLLER_TARGET_CHOICE_TAG => Some(TargetChoiceAttribution::AbilityController),
        OPPONENT_TARGET_CHOICE_TAG => Some(TargetChoiceAttribution::Opponent),
        _ => None,
    }
}

pub fn is_generated_internal_tag(tag: &str) -> bool {
    if let Some(rest) = tag.strip_prefix("__sentence_helper_") {
        let mut parts = rest.split("_l");
        let Some(_prefix) = parts.next() else {
            return false;
        };
        let Some(rest) = parts.next() else {
            return false;
        };
        let mut parts = rest.split("_s");
        let Some(line) = parts.next() else {
            return false;
        };
        let Some(rest) = parts.next() else {
            return false;
        };
        let mut parts = rest.split("_e");
        let Some(start) = parts.next() else {
            return false;
        };
        let Some(end) = parts.next() else {
            return false;
        };
        return parts.next().is_none()
            && !line.is_empty()
            && !start.is_empty()
            && !end.is_empty()
            && line.chars().all(|ch| ch.is_ascii_digit())
            && start.chars().all(|ch| ch.is_ascii_digit())
            && end.chars().all(|ch| ch.is_ascii_digit());
    }

    let Some((_, suffix)) = tag.rsplit_once('_') else {
        return false;
    };
    !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
}

pub fn is_implicit_reference_tag(tag: &str) -> bool {
    let action_root = tag.split('_').next().unwrap_or(tag);
    matches!(
        tag,
        "triggering"
            | "triggering_source"
            | "damaged"
            | "__it__"
            | crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG
            | crate::tag::SOURCE_EXILED_TAG
            | "other_attacker"
            | "blocking"
            | "searched_face_down" // "<verbed>_this_way" helper tags back-reference an object the same
                                   // sentence just described; oracle refers to it with a plain pronoun.
    ) || tag.starts_with("__sentence_helper_")
        || tag.ends_with("_this_way")
        // These tags are execution identities produced by selection and
        // result effects. Their labels are never oracle text, even in small
        // hand-built fixtures that omit the usual numeric suffix.
        || matches!(
            action_root,
            "chosen"
                | "selected"
                | "targeted"
                | "created"
                | "returned"
                | "exiled"
                | "looked"
                | "searched"
                | "revealed"
                | "matched"
                | "moved"
                | "sacrificed"
                | "destroyed"
                | "countered"
                | "discarded"
                | "milled"
                | "damaged"
        )
        || is_generated_internal_tag(tag)
}

pub fn choose_spec_is_plural(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. } => choose_spec_is_plural(spec),
        ChooseSpec::Target(inner) => choose_spec_is_plural(inner),
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _) => {
            if !count.dynamic_x && count.max == Some(1) {
                false
            } else {
                !count.is_single() || choose_spec_is_plural(inner)
            }
        }
        _ => false,
    }
}

fn strip_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

fn describe_each_other_filter(filter: &ObjectFilter) -> (String, bool) {
    let description = filter.description();
    if filter.source_surface.is_some() {
        let rest = strip_article(&description).trim();
        if rest.is_empty() {
            return ("each object".to_string(), false);
        }
        return (format!("each {rest}"), false);
    }

    let rest = description
        .strip_prefix("another ")
        .unwrap_or(description.as_str())
        .trim();
    let rest = strip_article(rest).trim();
    if rest.is_empty() {
        ("each other object".to_string(), false)
    } else {
        (format!("each other {rest}"), false)
    }
}

fn demonstrative_reference_plurality(text: &str) -> Option<bool> {
    if text == "it" || text.starts_with("that ") {
        Some(false)
    } else if text == "them" || text.starts_with("those ") {
        Some(true)
    } else {
        None
    }
}

fn attached_reference_surface_is_singular(text: &str) -> bool {
    text.starts_with("enchanted ") || text.starts_with("equipped ")
}

pub fn describe_apply_continuous_target<FChooseSpec, FPluralizeFilter>(
    effect: &ApplyContinuousEffect,
    describe_choose_spec: FChooseSpec,
    describe_plural_filter: FPluralizeFilter,
) -> (String, bool)
where
    FChooseSpec: Fn(&ChooseSpec) -> String,
    FPluralizeFilter: Fn(&ObjectFilter) -> String,
{
    if matches!(
        effect.target,
        EffectTarget::AllPermanents | EffectTarget::AllCreatures
    ) && let Some(spec @ ChooseSpec::Object(filter)) = &effect.target_spec
    {
        let described = describe_choose_spec(spec);
        if let Some(is_plural) = demonstrative_reference_plurality(&described) {
            return (described, is_plural);
        }
        if attached_reference_surface_is_singular(&described) {
            return (described, false);
        }
        if filter.other {
            return describe_each_other_filter(filter);
        }

        let description = filter.description();
        let rest = strip_article(&description).trim();
        if rest.is_empty() {
            return ("each object".to_string(), false);
        }

        return (format!("each {rest}"), false);
    }

    if matches!(effect.target, EffectTarget::Filter(_))
        && let Some(spec @ ChooseSpec::Object(filter)) = &effect.target_spec
    {
        let described = describe_choose_spec(spec);
        if let Some(is_plural) = demonstrative_reference_plurality(&described) {
            return (described, is_plural);
        }
        if attached_reference_surface_is_singular(&described) {
            return (described, false);
        }
        if filter.other {
            return describe_each_other_filter(filter);
        }

        let description = filter.description();
        let rest = strip_article(&description).trim();
        if rest.is_empty() {
            return ("each object".to_string(), false);
        }

        return (format!("each {rest}"), false);
    }

    if let Some(spec) = &effect.target_spec {
        let described = describe_choose_spec(spec);
        if let Some(is_plural) = demonstrative_reference_plurality(&described) {
            return (described, is_plural);
        }
        if attached_reference_surface_is_singular(&described) {
            return (described, false);
        }
        return (described, choose_spec_is_plural(spec));
    }

    match &effect.target {
        EffectTarget::Specific(_) => ("that permanent".to_string(), false),
        EffectTarget::Filter(filter) => {
            if filter.other {
                describe_each_other_filter(filter)
            } else {
                let described = filter.description();
                if attached_reference_surface_is_singular(&described) {
                    (described, false)
                } else {
                    (describe_plural_filter(filter), true)
                }
            }
        }
        EffectTarget::Source => ("this source".to_string(), false),
        EffectTarget::AllPermanents => ("all permanents".to_string(), true),
        EffectTarget::AllCreatures => ("all creatures".to_string(), true),
        EffectTarget::AttachedTo(_) => ("the attached permanent".to_string(), false),
    }
}

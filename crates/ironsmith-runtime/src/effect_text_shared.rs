use crate::continuous::EffectTarget;
use crate::effects::ApplyContinuousEffect;
use crate::filter::ObjectFilter;
use crate::target::ChooseSpec;

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
    matches!(tag, "triggering" | "damaged" | "__it__") || is_generated_internal_tag(tag)
}

pub fn choose_spec_is_plural(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner) => choose_spec_is_plural(inner),
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        ChooseSpec::WithCount(inner, count) => !count.is_single() || choose_spec_is_plural(inner),
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
    ) && let Some(ChooseSpec::Object(filter)) = &effect.target_spec
    {
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
        return (describe_choose_spec(spec), choose_spec_is_plural(spec));
    }

    match &effect.target {
        EffectTarget::Specific(_) => ("that permanent".to_string(), false),
        EffectTarget::Filter(filter) => {
            if filter.other {
                describe_each_other_filter(filter)
            } else {
                (describe_plural_filter(filter), true)
            }
        }
        EffectTarget::Source => ("this source".to_string(), false),
        EffectTarget::AllPermanents => ("all permanents".to_string(), true),
        EffectTarget::AllCreatures => ("all creatures".to_string(), true),
        EffectTarget::AttachedTo(_) => ("the attached permanent".to_string(), false),
    }
}

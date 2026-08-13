use super::*;

pub(crate) fn compile_choose_objects_with_subject(
    subject: LoweredSubject,
    filter: ObjectFilter,
    count: ChoiceCount,
    count_value: Option<Value>,
    tag: TagKey,
    zone: Zone,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let chooser = subject.as_chooser();
    let choose_effect = crate::effects::ChooseObjectsEffect::new(filter, count, chooser, tag)
        .with_count_value_opt(count_value)
        .in_zone(zone);
    let effects = subject.prepend_target_prelude_if_needed(Effect::new(choose_effect));
    (effects, subject.into_choices())
}

pub(crate) fn compile_choose_objects_across_zones_with_subject(
    subject: LoweredSubject,
    filter: ObjectFilter,
    count: ChoiceCount,
    count_value: Option<Value>,
    tag: TagKey,
    zones: Vec<Zone>,
    search_mode: Option<crate::effect::SearchSelectionMode>,
    default_search: bool,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let chooser = subject.as_chooser();
    let mut choose_effect = crate::effects::ChooseObjectsEffect::new(filter, count, chooser, tag)
        .with_count_value_opt(count_value)
        .in_zones(zones);
    if let Some(search_mode) = search_mode {
        choose_effect = match search_mode {
            crate::effect::SearchSelectionMode::Exact => choose_effect.as_search(),
            crate::effect::SearchSelectionMode::Optional => choose_effect.as_optional_search(),
            crate::effect::SearchSelectionMode::AllMatching => {
                choose_effect.as_all_matching_search()
            }
        };
    } else if default_search {
        choose_effect = choose_effect.as_search();
    }
    let effects = subject.prepend_target_prelude_if_needed(Effect::new(choose_effect));
    (effects, subject.into_choices())
}

pub(crate) fn compile_choose_player_with_subject(
    subject: LoweredSubject,
    filter: PlayerFilter,
    tag: TagKey,
    random: bool,
    excluded_tags: Vec<TagKey>,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let chooser = subject.as_chooser();
    let mut choose_effect =
        crate::effects::ChoosePlayerEffect::new(chooser, filter, tag).excluding_tags(excluded_tags);
    if random {
        choose_effect = choose_effect.at_random();
    }
    let effects = subject.prepend_target_prelude_if_needed(Effect::new(choose_effect));
    (effects, subject.into_choices())
}

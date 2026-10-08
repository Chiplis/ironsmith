use super::*;

const BASIC_LAND_TYPES: [Subtype; 5] = [
    Subtype::Plains,
    Subtype::Island,
    Subtype::Swamp,
    Subtype::Mountain,
    Subtype::Forest,
];

/// "choose a land of each basic land type, then destroy those lands"
/// (Sundering Titan, Planar Overlay): the lowering is five single choices
/// sharing one result tag, one per basic land type in Plains–Forest order.
/// Render that run as the authored per-type quantifier instead of five
/// separate sentences.
pub(crate) fn describe_choose_land_of_each_basic_land_type(effects: &[Effect]) -> Option<String> {
    let run = effects.get(..BASIC_LAND_TYPES.len())?;
    let chooses = run
        .iter()
        .map(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        })
        .collect::<Option<Vec<_>>>()?;
    let first = chooses.first()?;
    if first.filter.subtypes.as_slice() != [Subtype::Plains] {
        return None;
    }
    let mut base = first.filter.clone();
    base.subtypes.clear();
    for (choose, subtype) in chooses.iter().zip(BASIC_LAND_TYPES) {
        let mut expected = base.clone();
        expected.subtypes.push(subtype);
        if choose.filter != expected
            || choose.tag != first.tag
            || choose.chooser != first.chooser
            || choose.count != crate::effect::ChoiceCount::exactly(1)
            || choose.count_value.is_some()
            || choose.is_search
        {
            return None;
        }
    }
    // Presentation only: name the land noun even when the lowered filter
    // relies on the basic land subtype to imply it.
    if !base.card_types.contains(&CardType::Land) {
        base.card_types.push(CardType::Land);
    }
    let mut synthetic = (*first).clone();
    synthetic.filter = base;
    let choice = describe_effect_list(&[Effect::new(synthetic)]);
    let choice = choice.trim().trim_end_matches('.');
    if !choice.contains(" land") {
        return None;
    }
    let choice = choice.replacen(" land", " land of each basic land type", 1);
    let rest = &effects[BASIC_LAND_TYPES.len()..];
    if rest.is_empty() {
        return Some(choice);
    }
    let rest = describe_effect_list(rest);
    let rest = rest.trim().trim_end_matches('.');
    Some(format!("{choice}, then {}", lowercase_first(rest)))
}

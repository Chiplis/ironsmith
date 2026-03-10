use crate::cost::TotalCost;
use crate::tag::TagKey;

pub(crate) fn find_first_sacrifice_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("sacrifice_cost_") {
            return Some(choose.tag.clone());
        }
    }
    None
}

pub(crate) fn find_last_exile_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    let mut found = None;
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("exile_cost_") {
            found = Some(choose.tag.clone());
        }
    }
    found
}

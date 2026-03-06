use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::filter::TaggedOpbjectRelation;
use crate::tag::TagKey;
use crate::target::PlayerFilter;

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

pub(crate) fn normalize_alternative_cast_cost_effects_runtime(cost_effects: Vec<Effect>) -> Vec<Effect> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < cost_effects.len() {
        if idx + 1 < cost_effects.len()
            && let Some(choose) =
                cost_effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) =
                cost_effects[idx + 1].downcast_ref::<crate::effects::SacrificeEffect>()
            && sacrifice.player == PlayerFilter::You
        {
            let references_chosen = sacrifice.filter.tagged_constraints.len() == 1
                && sacrifice.filter.tagged_constraints[0].tag == choose.tag
                && sacrifice.filter.tagged_constraints[0].relation
                    == TaggedOpbjectRelation::IsTaggedObject;
            if references_chosen {
                out.push(Effect::sacrifice(
                    choose.filter.clone(),
                    sacrifice.count.clone(),
                ));
                idx += 2;
                continue;
            }
        }

        out.push(cost_effects[idx].clone());
        idx += 1;
    }

    out
}

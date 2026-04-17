pub type Cost = ironsmith_core::Cost<crate::effect::Effect>;

use ironsmith_core::CostComponent as _;

fn is_payment_effect(effect: &crate::effect::Effect) -> bool {
    use crate::effects;

    if effect.downcast_ref::<effects::PayManaEffect>().is_some()
        || effect.downcast_ref::<effects::TapEffect>().is_some()
        || effect.downcast_ref::<effects::UntapEffect>().is_some()
        || effect.downcast_ref::<effects::LoseLifeEffect>().is_some()
        || effect.downcast_ref::<effects::PayEnergyEffect>().is_some()
        || effect.downcast_ref::<effects::DiscardEffect>().is_some()
        || effect
            .downcast_ref::<effects::DiscardHandEffect>()
            .is_some()
        || effect.downcast_ref::<effects::MillEffect>().is_some()
        || effect.downcast_ref::<effects::SacrificeEffect>().is_some()
        || effect
            .downcast_ref::<effects::SacrificePlayerEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::SacrificeTargetEffect>()
            .is_some()
        || effect.downcast_ref::<effects::ExileEffect>().is_some()
        || effect
            .downcast_ref::<effects::ExileTopOfLibraryEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::ReturnToHandEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::RemoveCountersEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::RemoveAnyCountersAmongEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::RemoveAnyCountersFromSourceEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::PutCountersEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::ChooseObjectsEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::ChoosePlayerEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::ChooseCreatureTypeEffect>()
            .is_some()
        || effect.downcast_ref::<effects::BeholdEffect>().is_some()
        || effect
            .downcast_ref::<effects::RevealTaggedEffect>()
            .is_some()
        || effect.downcast_ref::<effects::CrewCostEffect>().is_some()
        || effect
            .downcast_ref::<effects::ConspireCostEffect>()
            .is_some()
        || effect
            .downcast_ref::<effects::NinjutsuCostEffect>()
            .is_some()
        || effect.downcast_ref::<effects::ExertCostEffect>().is_some()
        || effect
            .downcast_ref::<effects::EmitKeywordActionEffect>()
            .is_some()
    {
        return true;
    }

    if let Some(tagged) = effect.downcast_ref::<effects::TaggedEffect>() {
        return is_payment_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<effects::WithIdEffect>() {
        return is_payment_effect(&with_id.effect);
    }
    if let Some(may) = effect.downcast_ref::<effects::MayEffect<crate::effect::Effect>>() {
        return may.effects.iter().all(is_payment_effect);
    }
    if let Some(unless) =
        effect.downcast_ref::<effects::UnlessActionEffect<crate::effect::Effect>>()
    {
        return unless.effects.iter().all(is_payment_effect)
            && unless.alternative.iter().all(is_payment_effect);
    }
    if let Some(choice) = effect.downcast_ref::<effects::ChooseModeEffect>() {
        return choice
            .modes
            .iter()
            .all(|mode| mode.effects.iter().all(is_payment_effect));
    }

    false
}

pub(crate) fn payment_effect_to_cost(effect: crate::effect::Effect) -> Result<Cost, String> {
    if is_payment_effect(&effect) {
        Ok(Cost::effect(effect))
    } else {
        Err(format!(
            "effect is not marked as cost-executable: {}",
            effect.payload_type_name()
        ))
    }
}

pub(crate) fn payment_effects_to_total_cost(
    effects: impl IntoIterator<Item = crate::effect::Effect>,
) -> Result<crate::cost::TotalCost, String> {
    effects
        .into_iter()
        .map(payment_effect_to_cost)
        .collect::<Result<Vec<_>, _>>()
        .map(crate::cost::TotalCost::from_costs)
}

pub(crate) fn cost_to_payment_effect(cost: &Cost) -> Option<crate::effect::Effect> {
    match cost {
        Cost::Mana(mana_cost) => Some(crate::effect::Effect::new(
            crate::effects::PayManaEffect::new(
                mana_cost.clone(),
                crate::target::ChooseSpec::SourceController,
            ),
        )),
        Cost::Tap => Some(crate::effect::Effect::tap(
            crate::target::ChooseSpec::Source,
        )),
        Cost::Untap => Some(crate::effect::Effect::untap(
            crate::target::ChooseSpec::Source,
        )),
        Cost::DiscardSource => None,
        Cost::SacrificeSelf => Some(crate::effect::Effect::sacrifice_source()),
        Cost::Sacrifice(filter) => Some(crate::effect::Effect::sacrifice(filter.clone(), 1)),
        Cost::Discard { count, card_types } => {
            let filter = if card_types.is_empty() {
                None
            } else {
                let mut filter = crate::target::ObjectFilter::default();
                filter.card_types = card_types.clone();
                filter.zone = Some(crate::zone::Zone::Hand);
                Some(filter)
            };
            Some(crate::effect::Effect::discard_player_filtered(
                *count,
                crate::target::PlayerFilter::You,
                false,
                filter,
            ))
        }
        Cost::DiscardHand => Some(crate::effect::Effect::discard_hand()),
        Cost::RemoveCounters {
            counter_type,
            count,
        } => Some(crate::effect::Effect::remove_counters(
            *counter_type,
            *count,
            crate::target::ChooseSpec::Source,
        )),
        Cost::AddCounters {
            counter_type,
            count,
        } => Some(crate::effect::Effect::put_counters_on_source(
            *counter_type,
            *count as i32,
        )),
        Cost::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
        } => Some(crate::effect::Effect::new(
            crate::effects::RemoveAnyCountersFromSourceEffect {
                counter_type: *counter_type,
                display_x: *display_x,
            },
        )),
        Cost::Energy(amount) => Some(crate::effect::Effect::new(
            crate::effects::PayEnergyEffect::new(
                amount.clone(),
                crate::target::ChooseSpec::SourceController,
            ),
        )),
        Cost::Mill(count) => Some(crate::effect::Effect::mill_player(
            count.clone(),
            crate::target::PlayerFilter::You,
        )),
        Cost::Life(amount) => Some(crate::effect::Effect::lose_life_player(
            amount.clone(),
            crate::target::PlayerFilter::You,
        )),
        Cost::ExileSelf => Some(crate::effect::Effect::exile(
            crate::target::ChooseSpec::Source,
        )),
        Cost::ExileFromHand {
            count,
            color_filter,
        } => {
            let mut filter = crate::target::ObjectFilter::default()
                .in_zone(crate::zone::Zone::Hand)
                .owned_by(crate::target::PlayerFilter::You);
            if let Some(colors) = color_filter {
                filter = filter.with_colors(*colors);
            }
            Some(crate::effect::Effect::exile(
                crate::target::ChooseSpec::Object(filter)
                    .with_count(crate::effect::ChoiceCount::exactly(*count as usize)),
            ))
        }
        Cost::ReturnSelfToHand => Some(crate::effect::Effect::return_to_hand(
            crate::target::ChooseSpec::Source,
        )),
        Cost::Effect(effect) => Some(effect.clone()),
    }
}

pub(crate) fn total_cost_to_payment_effects(
    total_cost: &crate::cost::TotalCost,
) -> Vec<crate::effect::Effect> {
    total_cost
        .costs()
        .iter()
        .map(|cost| {
            cost_to_payment_effect(cost)
                .unwrap_or_else(|| panic!("unsupported cost component: {}", cost.display()))
        })
        .collect()
}

pub type Cost = ironsmith_core::Cost<crate::effect::Effect>;

use ironsmith_core::CostComponent as _;

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

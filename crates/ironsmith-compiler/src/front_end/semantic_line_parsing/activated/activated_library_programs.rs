use super::*;

pub(super) fn mark_forecast_reveal_duration(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| {
        Ok::<_, std::convert::Infallible>(match component {
            crate::model::CompilerCost::RevealSourceFromHand => {
                crate::model::CompilerCost::RevealSourceFromHandUntilUpkeepEnds
            }
            other => other,
        })
    })
    .expect("mapping a Forecast reveal cost is infallible")
}

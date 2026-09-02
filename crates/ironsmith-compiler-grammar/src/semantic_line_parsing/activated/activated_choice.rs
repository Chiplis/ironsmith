use super::*;
use crate::cards::builders::PredicateAst;

pub(super) fn apply_chosen_option_condition_to_activated(
    parsed: &mut ParsedAbility,
    chosen_option: Option<&ChosenOptionContext>,
) {
    let Some(context) = chosen_option else {
        return;
    };
    let condition = condition_for_chosen_option(context);
    let AbilityKind::Activated(activated) = parsed.kind_mut() else {
        return;
    };
    activated.activation_condition = Some(match activated.activation_condition.take() {
        Some(existing) => PredicateAst::And(Box::new(existing), Box::new(condition)),
        None => condition,
    });
    if let Some(threshold) = context.station_threshold() {
        // Renderer-only surface metadata derived from the typed station fact;
        // no later stage parses Oracle text to recover this threshold.
        activated
            .additional_restrictions
            .push(format!("__ironsmith_station_threshold:{threshold}"));
    }
}

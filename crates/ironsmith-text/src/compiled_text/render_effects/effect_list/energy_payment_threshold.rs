use super::*;

/// A numeric result test immediately following an energy payment names the
/// amount paid, rather than merely whether a zero-allowed payment succeeded.
pub(super) fn describe_energy_payment_threshold(effects: &[Effect]) -> Option<String> {
    // Saga and other triggered programs begin with a textless event capture.
    let effects = if let [first, rest @ ..] = effects
        && first.downcast_ref::<crate::effects::TagTriggeringObjectEffect>().is_some()
    { rest } else { effects };
    let [payment, branch, rest @ ..] = effects else { return None; };
    let payment_id = wrapped_with_id(payment)?;
    let pay = unwrap_wrapped_effect(payment)
        .downcast_ref::<crate::effects::PayAnyEnergyEffect>()?;
    if !matches!(pay.player, ChooseSpec::Player(PlayerFilter::You)) {
        return None;
    }
    let branch = unwrap_wrapped_effect(branch).downcast_ref::<crate::effects::IfEffect>()?;
    if branch.condition != payment_id.id || !branch.else_.is_empty() || branch.per_player_result {
        return None;
    }
    let threshold = match branch.predicate {
        EffectPredicate::Value(Comparison::GreaterThan(0)) => 1,
        EffectPredicate::Value(Comparison::GreaterThanOrEqual(n)) if n > 0 => n,
        _ => return None,
    };
    let threshold = if threshold == 1 { "one".to_string() } else { threshold.to_string() };
    let mut text = format!(
        "{}. If you paid {threshold} or more {{E}} this way, {}",
        describe_effect(payment), lowercase_first(&describe_effect_list(&branch.then)),
    );
    if !rest.is_empty() {
        text.push_str(". ");
        text.push_str(&describe_effect_list(rest));
    }
    Some(text)
}

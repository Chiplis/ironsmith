use super::*;

fn resource_player(effect: &Effect) -> Option<&PlayerFilter> {
    let effect = structural_unwrap_render_wrappers(effect);
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        let ChooseSpec::Player(player) = lose.player.unhinted() else {
            return None;
        };
        return Some(player);
    }
    if let Some(poison) = effect.downcast_ref::<crate::effects::PoisonCountersEffect>() {
        return Some(&poison.player);
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        return Some(&mill.player);
    }
    None
}

fn player_reference_matches(
    player: &PlayerFilter,
    declared: &PlayerFilter,
    first_action: bool,
) -> bool {
    match player {
        PlayerFilter::Target(inner) if first_action => inner.as_ref() == declared,
        PlayerFilter::AliasedTarget(inner) if !first_action => inner.as_ref() == declared,
        _ => false,
    }
}

/// Reconstruct one target-player action whose comma-then followups carry the
/// same typed player identity. Lowering retains the target declaration for
/// gameplay and changes later references to `AliasedTarget`; rendering must
/// elide those anaphoric subjects without creating additional targets.
pub(in crate::compiled_text) fn describe_target_player_resource_coordination(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, actions @ ..] = effects else {
        return None;
    };
    if actions.len() < 2 {
        return None;
    }
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.explicit_declaration || target.chooser.is_some() || !target.target.is_target() {
        return None;
    }
    let ChooseSpec::Player(declared) = target.target.base() else {
        return None;
    };
    let subject = match declared {
        PlayerFilter::Opponent => "target opponent",
        PlayerFilter::Any => "target player",
        _ => return None,
    };

    let mut clauses = Vec::with_capacity(actions.len());
    for (index, effect) in actions.iter().enumerate() {
        let player = resource_player(effect)?;
        if !player_reference_matches(player, declared, index == 0) {
            return None;
        }
        let rendered = describe_effect(effect);
        let rendered = rendered.trim().trim_end_matches('.');
        let prefix = if index == 0 { subject } else { "that player" };
        let tail = rendered.strip_prefix(prefix)?.trim_start();
        if tail.is_empty() || tail.contains(". ") {
            return None;
        }
        clauses.push(if index == 0 {
            format!("{subject} {tail}")
        } else {
            tail.to_string()
        });
    }

    let last = clauses.pop()?;
    Some(capitalize_first(&format!(
        "{}, then {last}",
        clauses.join(", ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinated_effects() -> Vec<Effect> {
        let target = PlayerFilter::Opponent;
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Target(
                Box::new(ChooseSpec::Player(target.clone())),
            ))),
            Effect::new(crate::effects::LoseLifeEffect::with_filter(
                2,
                PlayerFilter::Target(Box::new(target.clone())),
            )),
            Effect::new(crate::effects::PoisonCountersEffect::new(
                1,
                PlayerFilter::AliasedTarget(Box::new(target.clone())),
            )),
            Effect::new(crate::effects::MillEffect::new(
                6,
                PlayerFilter::AliasedTarget(Box::new(target)),
            )),
        ]
    }

    #[test]
    fn shared_target_player_resource_actions_fold() {
        let effects = coordinated_effects();
        let rendered = effects.iter().map(describe_effect).collect::<Vec<_>>();
        assert_eq!(
            describe_target_player_resource_coordination(&effects).as_deref(),
            Some("Target opponent loses 2 life, gets a poison counter, then mills six cards"),
            "{rendered:?}"
        );
    }

    #[test]
    fn changed_followup_target_is_not_folded() {
        let mut effects = coordinated_effects();
        effects[2] = Effect::new(crate::effects::PoisonCountersEffect::new(
            1,
            PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)),
        ));

        assert!(describe_target_player_resource_coordination(&effects).is_none());
    }
}

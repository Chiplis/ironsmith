use super::*;

/// Preserve the noun selected by an earlier player-or-planeswalker target
/// when the opposite coin-flip branch counts permanents controlled by that
/// player or that planeswalker's controller.
///
/// The count filter deliberately stores one semantic player relation for both
/// "that player" and "that opponent" surfaces. The preceding typed damage
/// target supplies the missing authored noun without relying on card identity
/// or rendered-text replacement.
pub(super) fn describe_player_or_planeswalker_backref_count_damage(
    target_branch: &crate::effects::IfEffect,
    backref_branch: &crate::effects::IfEffect,
) -> Option<String> {
    if target_branch.condition != backref_branch.condition
        || target_branch.predicate != EffectPredicate::Happened
        || backref_branch.predicate != EffectPredicate::DidNotHappen
        || !target_branch.else_.is_empty()
        || !backref_branch.else_.is_empty()
    {
        return None;
    }

    let target_damage = single_damage_effect_view(&target_branch.then)?;
    let referenced_player = match target_damage.target.base() {
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Opponent) => "opponent",
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any) => "player",
        _ => return None,
    };
    if target_damage.source_is_combat || target_damage.unpreventable {
        return None;
    }

    let backref_damage = single_damage_effect_view(&backref_branch.then)?;
    if backref_damage.source_is_combat
        || backref_damage.unpreventable
        || !backref_damage
            .amount
            .has_surface_hint(ValueSurfaceHint::EqualTo)
    {
        return None;
    }
    let Value::Count(count_filter) = backref_damage.amount.unhinted() else {
        return None;
    };
    if count_filter.zone != Some(Zone::Battlefield)
        || count_filter.controller != Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    {
        return None;
    }

    let mut counted_subject = count_filter.clone();
    counted_subject.zone = None;
    counted_subject.controller = None;
    let counted_subject = describe_count_filter_value_subject(&counted_subject);

    Some(format!(
        "Deal damage to {} equal to the number of {counted_subject} that {referenced_player} or that planeswalker's controller controls",
        describe_damage_target(&backref_damage.target)
    ))
}

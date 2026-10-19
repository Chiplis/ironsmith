//! Recover an optional recipient substitution from the executable choice branches.
use super::*;
use super::helpers_00::unwrap_render_wrappers;

pub(super) fn describe(effects: &[&Effect]) -> Option<String> {
    let [declaration, optional, branch] = effects else { return None; };
    let declaration = unwrap_render_wrappers(declaration)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let optional = unwrap_render_wrappers(optional)
        .downcast_ref::<crate::effects::MayEffect>()?;
    let branch = unwrap_render_wrappers(branch)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [choice] = optional.effects.as_slice() else { return None; };
    let choice = unwrap_render_wrappers(choice)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let [redirected] = branch.if_true.as_slice() else { return None; };
    let [original] = branch.if_false.as_slice() else { return None; };
    let redirected = unwrap_render_wrappers(redirected)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let original = unwrap_render_wrappers(original)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let chooser = PlayerFilter::TargetPlayerOrControllerOfTarget;
    if !matches!(declaration.target.base(), ChooseSpec::PlayerOrPlaneswalker(_))
        || declaration.target.base() != original.target.base()
        || optional.decider.as_ref() != Some(&chooser)
        || optional.pay_as_cost
        || optional.fallback != crate::decision::FallbackStrategy::Decline
        || choice.chooser != chooser
        || choice.count != crate::effect::ChoiceCount::exactly(1)
        || choice.count_value.is_some()
        || choice.aggregate_constraint.is_some()
        || choice.is_search || choice.reveal || choice.top_only || choice.bottom_only
        || choice.remember_as_chosen_object || choice.replace_tagged_objects
        || !choice.additional_zones.is_empty()
        || choice.filter.controller.as_ref() != Some(&chooser)
        || branch.condition != Condition::TaggedObjectMatches(choice.tag.clone(), ObjectFilter::default())
        || redirected.target != ChooseSpec::Tagged(choice.tag.clone())
        || original.source_is_combat
    { return None; }
    // All damage properties must agree; only the recipient may change.
    let mut same_damage = redirected.clone();
    same_damage.target = original.target.clone();
    if same_damage != *original { return None; }
    let original_text = describe_effect(&Effect::new(original.clone()));
    let chosen = describe_choose_selection(choice);
    Some(format!("{original_text}. That player or that planeswalker's controller may choose {chosen} and have this spell deal that damage to it instead"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_damage_recipient_keeps_the_exclusive_instead_clause() {
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Recipient spell")
            .card_types(vec![crate::types::CardType::Instant])
            .parse_text("Recipient spell deals X damage to target player or planeswalker. That player or that planeswalker's controller may choose a creature they control and have Recipient spell deal that damage to it instead.")
            .expect("recipient choice compiles");
        let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(text.contains("damage to target player or planeswalker"), "{text}");
        assert!(text.contains("may choose") && text.contains("that damage to it instead"), "{text}");
        assert!(!text.contains("damage_recipient_choice") && !text.contains("If tagged"), "{text}");
        let effects: Vec<_> = definition.spell_effect.as_ref().unwrap().all_effects();
        assert!(describe(&effects).is_some());
        let mut branch = unwrap_render_wrappers(effects[2])
            .downcast_ref::<crate::effects::ConditionalEffect>().unwrap().clone();
        let mut damage = unwrap_render_wrappers(&branch.if_true[0])
            .downcast_ref::<crate::effects::DealDamageEffect>().unwrap().clone();
        damage.amount = crate::effect::Value::Fixed(7);
        branch.if_true = vec![Effect::new(damage)];
        let changed = Effect::new(branch);
        assert!(describe(&[effects[0], effects[1], &changed]).is_none(),
            "a changed damage amount is not a recipient-only substitution");
    }
}

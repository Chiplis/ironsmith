use super::*;

/// Render the exact ETB payment-failure program as its authored `unless`
/// clause.
///
/// The executable condition is the negation of "at least one colored mana was
/// spent" and the only branch sacrifices the triggering object. Keeping that
/// precise condition/action/tag correlation lets compiled text recover the
/// concise Oracle surface without weakening arbitrary negative conditions.
pub(super) fn describe_sacrifice_triggering_unless_mana_spent(
    effects: &[Effect],
) -> Option<String> {
    let [tag_root, conditional_root] = effects else {
        return None;
    };
    let tag = structural_unwrap_render_wrappers(tag_root)
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let conditional = structural_unwrap_render_wrappers(conditional_root)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let Condition::Not(inner) = &conditional.condition else {
        return None;
    };
    let Condition::ManaSpentToCastThisSpellAtLeast {
        amount: 1,
        symbol: Some(symbol),
    } = inner.as_ref()
    else {
        return None;
    };
    let [sacrifice_root] = conditional.if_true.as_slice() else {
        return None;
    };
    let sacrifice = structural_unwrap_render_wrappers(sacrifice_root)
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if sacrifice.target != ChooseSpec::Tagged(tag.tag.clone()) {
        return None;
    }

    Some(format!(
        "Sacrifice it unless {} was spent to cast it",
        describe_mana_symbol(*symbol)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(amount: u32, sacrifice_tag: &str) -> Vec<Effect> {
        let tag = TagKey::from("triggering");
        vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new(tag)),
            Effect::new(crate::effects::ConditionalEffect::new(
                Condition::Not(Box::new(Condition::ManaSpentToCastThisSpellAtLeast {
                    amount,
                    symbol: Some(crate::mana::ManaSymbol::Red),
                })),
                vec![Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::Tagged(TagKey::from(sacrifice_tag)),
                ))],
                vec![],
            )),
        ]
    }

    #[test]
    fn sacrifice_unless_requires_exact_spend_threshold_and_trigger_tag() {
        assert_eq!(
            describe_sacrifice_triggering_unless_mana_spent(&effects(1, "triggering")),
            Some("Sacrifice it unless {R} was spent to cast it".to_string())
        );
        assert_eq!(
            describe_sacrifice_triggering_unless_mana_spent(&effects(2, "triggering")),
            None
        );
        assert_eq!(
            describe_sacrifice_triggering_unless_mana_spent(&effects(1, "other")),
            None
        );
    }
}

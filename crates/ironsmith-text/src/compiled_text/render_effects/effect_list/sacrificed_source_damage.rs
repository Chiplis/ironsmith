use super::*;
use crate::target::SourceReferenceSurface;

/// Preserve the authored last-known-information pronoun after the source is
/// sacrificed. The sequence remains fully executable: an optional synthetic
/// target declaration owns the damage target, the sacrifice owns the exact
/// `this creature` source surface, and `ExecuteWithSourceEffect` proves that
/// the sacrificed source is also the damage source.
pub(in crate::compiled_text::render_effects) fn describe_sacrificed_source_damage_backreference(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return None;
    }
    let (target, sacrifice_effect, damage_effect) = match sequence.effects.as_slice() {
        [sacrifice, damage] => (None, sacrifice, damage),
        [target, sacrifice, damage] => {
            let target = target.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            if target.explicit_declaration || target.chooser.is_some() {
                return None;
            }
            (Some(target), sacrifice, damage)
        }
        _ => return None,
    };

    let sacrifice = sacrifice_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target.base(), ChooseSpec::Source)
        || !matches!(
            sacrifice.target.source_reference_surface(),
            Some(SourceReferenceSurface::ThisPermanentType(surface))
                if surface.trim().eq_ignore_ascii_case("this creature")
        )
    {
        return None;
    }
    let execute = damage_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(execute.source.base(), ChooseSpec::Source) {
        return None;
    }
    let damage = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if let Some(target) = target
        && !target_specs_select_same_objects(&target.target, &damage.target)
    {
        return None;
    }

    let sacrifice_text = describe_effect(sacrifice_effect);
    let damage_text = describe_effect(damage_effect);
    let predicate = damage_text
        .strip_prefix("this creature ")
        .or_else(|| damage_text.strip_prefix("This creature "))
        .or_else(|| damage_text.strip_prefix("it "))
        .or_else(|| damage_text.strip_prefix("It "))?;
    Some(format!(
        "{} and it {predicate}",
        sacrifice_text.trim().trim_end_matches('.')
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "When another creature enters, sacrifice this creature and it deals 3 damage to target player or planeswalker.";

    fn parsed_sequence() -> crate::effects::SequenceEffect {
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Mogg Bombers Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(LINE)
        .expect("source-sacrifice damage trigger should compile");
        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability")
        };
        triggered.effects.segments[0]
            .default_effects
            .iter()
            .find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::SequenceEffect>()
                    .cloned()
            })
            .expect("coordinated sacrifice-damage sequence")
    }

    #[test]
    fn public_trigger_keeps_the_sacrificed_source_pronoun() {
        let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Mogg Bombers Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(LINE)
        .expect("source-sacrifice damage trigger should compile");
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            [LINE]
        );
    }

    #[test]
    fn different_damage_source_or_declared_target_is_not_compacted() {
        let sequence = parsed_sequence();

        let mut wrong_source = sequence.clone();
        let execute = wrong_source.effects[2]
            .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
            .expect("damage source")
            .clone();
        wrong_source.effects[2] = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::tagged("different_source"),
            *execute.effect,
        ));
        assert!(describe_sacrificed_source_damage_backreference(&wrong_source).is_none());

        let mut explicit_target = sequence;
        let target = explicit_target.effects[0]
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .expect("synthetic target")
            .clone();
        explicit_target.effects[0] = Effect::new(crate::effects::TargetOnlyEffect {
            explicit_declaration: true,
            ..target
        });
        assert!(describe_sacrificed_source_damage_backreference(&explicit_target).is_none());
    }
}

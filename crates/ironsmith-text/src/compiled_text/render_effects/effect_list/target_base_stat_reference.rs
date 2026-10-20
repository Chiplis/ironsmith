use super::*;

/// A target declaration can supply the value used to set the source's base
/// characteristic. Keep the measured object's identity instead of letting
/// the standalone continuous-effect renderer say "its power" for the source.
pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    if let Some(text) = describe_untagged_target_stat(effects) {
        return Some(text);
    }
    let [producer, consumer] = effects else { return None; };
    let producer = producer.downcast_ref::<crate::effects::TaggedEffect>()?;
    if producer.outcome_only { return None; }
    let target = producer.effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target.explicit_declaration || target.chooser.is_some() || !target.target.is_target() {
        return None;
    }
    let applied = consumer.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let source = applied.target_spec.as_ref()?;
    let source_noun = describe_choose_spec(source);
    if !matches!(source.unhinted(), ChooseSpec::Source)
        || !source_noun.starts_with("this ")
        || applied.target != crate::continuous::EffectTarget::Source
        || !applied.additional_modifications.is_empty()
        || !applied.runtime_modifications.is_empty()
        || applied.condition.is_some()
        || applied.source_type.is_some()
        || applied.source_reference_surface.is_some()
        || applied.set_quantifier_surface.is_some()
        || applied.type_retention_surface.is_some()
        || applied.animation_pt_surface.is_some()
        || applied.animation_duration_surface.is_some()
        || applied.lock_filter_at_resolution
        || !applied.resolve_set_pt_values_at_resolution
        || !applied.require_creature_target
        || applied.grant_only_if_absent
    { return None; }
    if let Some(crate::continuous::Modification::SetPowerToughness {
        power, toughness, sublayer,
    }) = &applied.modification {
        if *sublayer != crate::continuous::PtSublayer::Setting
            || !matches!(power.unhinted(), Value::PowerOf(spec)
                if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag == &producer.tag))
            || !matches!(toughness.unhinted(), Value::ToughnessOf(spec)
                if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag == &producer.tag))
        {
            return None;
        }
        let measured_target = describe_choose_spec(&target.target);
        if measured_target.is_empty() { return None; }
        let duration = describe_until(&applied.until);
        let suffix = if duration.is_empty() { String::new() } else { format!(" {duration}") };
        return Some(format!(
            "{}'s base power and toughness become equal to the power and toughness of {measured_target}{suffix}",
            capitalize_first(&source_noun),
        ));
    }
    if applied.until != Until::Forever { return None; }
    let (stat, value) = match applied.modification.as_ref()? {
        crate::continuous::Modification::SetPower { value, sublayer }
            if *sublayer == crate::continuous::PtSublayer::Setting => ("power", value),
        crate::continuous::Modification::SetToughness { value, sublayer }
            if *sublayer == crate::continuous::PtSublayer::Setting => ("toughness", value),
        _ => return None,
    };
    let Value::Add(left, right) = value.unhinted() else { return None; };
    let (Value::Fixed(offset), measured) = (left.unhinted(), right.unhinted()) else {
        return None;
    };
    if *offset < 0 { return None; }
    let characteristic = match measured {
        Value::PowerOf(spec) if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag == &producer.tag) => "power",
        Value::ToughnessOf(spec) if matches!(spec.unhinted(), ChooseSpec::Tagged(tag) if tag == &producer.tag) => "toughness",
        _ => return None,
    };
    let measured_target = describe_choose_spec(&target.target);
    if measured_target.is_empty() { return None; }
    Some(format!(
        "{}'s base {stat} becomes equal to {offset} plus the {characteristic} of {measured_target}",
        capitalize_first(&source_noun),
    ))
}

/// "{1}{U}: This creature's base power becomes equal to target creature's
/// power." (Riptide Mangler): an implicit target declaration followed by an
/// indefinite base-stat set measured directly off that target.
fn describe_untagged_target_stat(effects: &[Effect]) -> Option<String> {
    let [producer, consumer] = effects else { return None; };
    let target = producer.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.chooser.is_some() || !target.target.is_target() {
        return None;
    }
    let applied = consumer.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let source = applied.target_spec.as_ref()?;
    let source_noun = describe_choose_spec(source);
    if !matches!(source.unhinted(), ChooseSpec::Source)
        || !source_noun.starts_with("this ")
        || applied.target != crate::continuous::EffectTarget::Source
        || applied.until != Until::Forever
        || !applied.additional_modifications.is_empty()
        || !applied.runtime_modifications.is_empty()
        || applied.condition.is_some()
        || applied.source_type.is_some()
        || applied.source_reference_surface.is_some()
        || applied.set_quantifier_surface.is_some()
        || applied.type_retention_surface.is_some()
        || applied.animation_pt_surface.is_some()
        || applied.animation_duration_surface.is_some()
        || applied.lock_filter_at_resolution
        || !applied.resolve_set_pt_values_at_resolution
        || applied.grant_only_if_absent
    {
        return None;
    }
    let (stat, value) = match applied.modification.as_ref()? {
        crate::continuous::Modification::SetPower { value, sublayer }
            if *sublayer == crate::continuous::PtSublayer::Setting => ("power", value),
        crate::continuous::Modification::SetToughness { value, sublayer }
            if *sublayer == crate::continuous::PtSublayer::Setting => ("toughness", value),
        _ => return None,
    };
    let (characteristic, measured) = match value.unhinted() {
        Value::PowerOf(spec) => ("power", spec),
        Value::ToughnessOf(spec) => ("toughness", spec),
        _ => return None,
    };
    if **measured != target.target {
        return None;
    }
    let measured_target = describe_choose_spec(&target.target);
    if !measured_target.starts_with("target ") {
        return None;
    }
    Some(format!(
        "{}'s base {stat} becomes equal to {measured_target}'s {characteristic}",
        capitalize_first(&source_noun),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORACLE: &str = "{0}: This creature's base toughness becomes equal to 1 plus the power of target creature blocking or blocked by this creature.";

    #[test]
    fn source_stat_uses_the_targeted_creatures_power() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Characteristic Probe")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .parse_text(ORACLE)
            .expect("target-linked base characteristic parses");
        let text = crate::compiled_text::compiled_text_lines(&definition).join("\n");
        assert!(text.contains("base toughness becomes equal to 1 plus the power of target creature blocking or blocked by this creature"), "{text}");
        assert!(!text.contains("base toughness 1 plus its power"), "{text}");
    }

    #[test]
    fn changed_measured_tag_does_not_claim_the_targeted_characteristic() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Characteristic Probe")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .parse_text(ORACLE)
            .expect("target-linked base characteristic parses");
        let crate::ability::AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
            panic!("activated characteristic setter");
        };
        let [segment] = activated.effects.segments.as_slice() else { panic!("one resolution segment"); };
        let [producer, consumer] = segment.default_effects.as_slice() else { panic!("target and setter"); };
        let mut applied = consumer.downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("continuous base characteristic setter").clone();
        applied.modification = Some(crate::continuous::Modification::SetToughness {
            value: Value::Add(
                Box::new(Value::Fixed(1)),
                Box::new(Value::PowerOf(Box::new(ChooseSpec::Tagged(crate::TagKey::from("other"))))),
            ),
            sublayer: crate::continuous::PtSublayer::Setting,
        });
        assert!(describe(&[producer.clone(), Effect::new(applied)]).is_none());
    }
}

#[cfg(test)]
mod paired_target_tests {
    use super::*;

    #[test]
    fn paired_base_stats_keep_measured_target_and_duration() {
        let oracle = "At the beginning of your upkeep, this creature's base power and toughness become equal to the power and toughness of target creature other than this creature until the end of your next upkeep.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Paired characteristic probe")
            .card_types(vec![CardType::Creature])
            .parse_text(oracle).expect("paired reference parses");
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected triggered setter");
        };
        let effects = triggered.effects.flattened_default_effects();
        let [producer, consumer] = effects else { panic!("target and setter"); };
        let mut applied = consumer.downcast_ref::<crate::effects::ApplyContinuousEffect>().unwrap().clone();
        let Some(crate::continuous::Modification::SetPowerToughness { toughness, .. }) = &mut applied.modification else {
            panic!("paired setting");
        };
        *toughness = Value::ToughnessOf(Box::new(ChooseSpec::Tagged(crate::TagKey::from("different_target"))));
        assert!(describe(&[producer.clone(), Effect::new(applied)]).is_none());
    }
}

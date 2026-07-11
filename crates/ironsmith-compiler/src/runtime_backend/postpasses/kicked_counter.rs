use crate::cards::CardDefinition;

pub(super) fn finalize_mana_value_replacement(
    mut definition: CardDefinition,
    should_rewrite: bool,
) -> CardDefinition {
    if !should_rewrite {
        return definition;
    }

    let Some(spell_effect) = definition.spell_effect.as_ref() else {
        return definition;
    };
    let [segment] = spell_effect.segments.as_slice() else {
        return definition;
    };
    let [base_branch] = segment.self_replacements.as_slice() else {
        return definition;
    };
    let crate::effect::Condition::TaggedObjectMatches(tag, base_filter) = &base_branch.condition
    else {
        return definition;
    };
    if !matches!(
        base_filter.mana_value.as_ref(),
        Some(crate::target::Comparison::LessThanOrEqual(value)) if *value == 2
    ) {
        return definition;
    }

    let mut kicked_filter = base_filter.clone();
    kicked_filter.mana_value = Some(crate::target::Comparison::LessThanOrEqual(4));

    let mut default_effects = segment.default_effects.clone();
    default_effects.push(crate::effect::Effect::conditional(
        base_branch.condition.clone(),
        base_branch.replacement_effects.clone(),
        Vec::new(),
    ));

    let kicked_effect = crate::effect::Effect::conditional(
        crate::effect::Condition::TaggedObjectMatches(tag.clone(), kicked_filter),
        base_branch.replacement_effects.clone(),
        Vec::new(),
    );
    let mut program = crate::resolution::ResolutionProgram::from_effects(default_effects);
    if let Some(segment) = program.last_segment_mut() {
        segment
            .self_replacements
            .push(crate::resolution::SelfReplacementBranch::new(
                crate::effect::Condition::ThisSpellWasKicked,
                vec![kicked_effect],
            ));
        definition.spell_effect = Some(program);
    }
    definition
}

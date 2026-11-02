//! Immutable, definition-local identity for activation history and retained text managers.
use crate::ability::AbilityKind;
use crate::cards::{CardDefinition, builders::CardTextError};
use sha2::{Digest, Sha256};

fn reads_activation_history(condition: &crate::effect::Condition) -> bool {
    use crate::effect::Condition;
    match condition {
        Condition::ThisAbilityActivatedThisTurnAtLeast(_) => true,
        Condition::Not(inner) => reads_activation_history(inner),
        Condition::And(left, right) | Condition::Or(left, right) =>
            reads_activation_history(left) || reads_activation_history(right),
        _ => false,
    }
}
fn effect_needs_activation_definition(effect: &crate::effect::Effect) -> bool {
    // A text-fragment manager keeps its acquired occurrence while its own
    // instructions are replaced by another precompiled subset. Its selected
    // body may change; its authored activation identity must not disappear.
    let mut reads = effect.downcast_ref::<ironsmith_core::RemoveOptionalTextFragmentEffect>().is_some()
        || effect.downcast_ref::<crate::effects::ConditionalEffect>()
            .is_some_and(|conditional| reads_activation_history(&conditional.condition));
    effect.visit_child_effects(&mut |child| reads |= effect_needs_activation_definition(child));
    reads
}
fn ability_needs_activation_definition(ability: &crate::ability::Ability) -> bool {
    let AbilityKind::Activated(activated) = &ability.kind else { return false; };
    // Designated abilities are queried as exact current occurrences. Merely
    // retaining their printed slot would alias another face's definition.
    !activated.effects.activation_designations.is_empty()
        || activated.effects.activation_label.is_some()
        || activated.effects.all_effects().into_iter().any(effect_needs_activation_definition)
        || activated.effects.segments.iter().any(|segment| segment.self_replacements.iter()
            .any(|branch| reads_activation_history(&branch.condition)))
        // "This ability can't be copied." A copy effect identifies the stack
        // entry's authored activation by this definition.
        || activated.ability_cant_be_copied()
}

/// The compiler, not a later live source lookup, owns this namespace. Include
/// the entire typed face and every ability's costs/instructions, then its
/// authored occurrence. Local allocator CardIds are deliberately excluded.
/// Card names are ordinary definition metadata here; they never select rules.
pub(super) fn stamp_activation_definitions(definition: &mut CardDefinition) -> Result<(), CardTextError> {
    stamp_activation_definitions_with_generated(definition, &[])
}

pub(super) fn stamp_activation_definitions_with_generated(
    definition: &mut CardDefinition,
    generated: &[ironsmith_core::LinkedExileDefinition],
) -> Result<(), CardTextError> {
    let readers: Vec<_> = definition.abilities.iter().map(ability_needs_activation_definition).collect();
    fn cause_reader(trigger: &ironsmith_core::CompilerTrigger) -> bool {
        match &trigger.kind {
            ironsmith_core::TriggerKind::CounterPutOn(counter) => counter.activation_label.is_some(),
            ironsmith_core::TriggerKind::AnyOf(branches) => branches.iter().any(cause_reader),
            _ => false,
        }
    }
    let has_cause_reader = definition.abilities.iter().any(|ability| match &ability.kind {
        AbilityKind::Triggered(triggered) => cause_reader(&triggered.trigger), _ => false,
    });
    if !readers.iter().any(|reader| *reader) && !has_cause_reader { return Ok(()); }
    let (bytes, domain): (_, &[u8]) = match super::trigger_definitions::authored_root_namespace_with_generated(definition, generated)? {
        Some(bytes) => (bytes, b"ironsmith-activated-definition-v2\0"),
        None => {
            // Activation history predates this text-identity increment. Keep
            // that existing owner available when an older opaque link stamp
            // prevents the stronger canonical graph proof. This fallback is
            // not new text-domain clearance or allocator-independence proof.
            let mut face = definition.card.clone();
            face.id = ironsmith_core::CardId::from_raw(0);
            face.other_face = None;
            face.first_printed_set_name = None;
            let mut abilities = definition.abilities.clone();
            for ability in &mut abilities {
                match &mut ability.kind {
                    AbilityKind::Activated(activated) => activated.effects.activation_definition = None,
                    AbilityKind::Triggered(triggered) => triggered.effects.trigger_definition = None,
                    AbilityKind::Static(_) => {}
                }
            }
            let bytes = serde_json::to_vec(&(face, abilities)).map_err(|error|
                CardTextError::InvariantViolation(format!("cannot retain legacy activation definition identity: {error}")))?;
            (bytes, b"ironsmith-activated-definition-v1\0")
        }
    };
    for (occurrence, ability) in definition.abilities.iter_mut().enumerate() {
        if readers[occurrence] && let AbilityKind::Activated(activated) = &mut ability.kind {
            let mut digest = Sha256::new();
            digest.update(domain);
            digest.update(&bytes);
            digest.update((occurrence as u64).to_le_bytes());
            activated.effects.activation_definition = Some(ironsmith_core::LinkedExileDefinition(digest.finalize().into()));
        }
    }
    // Resolve authored labels once. Missing/ambiguous producers are errors;
    // runtime never guesses from a current ability index or permanent marker.
    let mut labels = std::collections::BTreeMap::new();
    for ability in &definition.abilities {
        if let AbilityKind::Activated(activated) = &ability.kind
            && let Some(label) = &activated.effects.activation_label {
            if labels.insert(label.clone(), activated.effects.activation_definition).is_some() {
                return Err(CardTextError::ParseError(format!("ambiguous activated ability label '{label}'")));
            }
        }
    }
    fn bind(trigger: &mut ironsmith_core::CompilerTrigger, labels: &std::collections::BTreeMap<String, Option<ironsmith_core::LinkedExileDefinition>>) -> Result<(),CardTextError> {
        match &mut trigger.kind {
            ironsmith_core::TriggerKind::CounterPutOn(counter) => {
                if let Some(label) = &counter.activation_label {
                    counter.activation_definition = Some(labels.get(label).copied().flatten().ok_or_else(||
                        CardTextError::ParseError(format!("no authored activated ability labeled '{label}'")))?);
                }
            }
            ironsmith_core::TriggerKind::AnyOf(branches) => { for branch in branches { bind(branch,labels)?; } }
            _ => {}
        }
        Ok(())
    }
    for ability in &mut definition.abilities {
        if let AbilityKind::Triggered(triggered) = &mut ability.kind { bind(&mut triggered.trigger,&labels)?; }
    }
    Ok(())
}

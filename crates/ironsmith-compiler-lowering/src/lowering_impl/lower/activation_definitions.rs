//! Immutable, definition-local identity for activation history readers.
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
fn effect_reads_activation_history(effect: &crate::effect::Effect) -> bool {
    let mut reads = effect.downcast_ref::<crate::effects::ConditionalEffect>()
        .is_some_and(|conditional| reads_activation_history(&conditional.condition));
    effect.visit_child_effects(&mut |child| reads |= effect_reads_activation_history(child));
    reads
}
fn ability_reads_activation_history(ability: &crate::ability::Ability) -> bool {
    let AbilityKind::Activated(activated) = &ability.kind else { return false; };
    activated.effects.all_effects().into_iter().any(effect_reads_activation_history)
        || activated.effects.segments.iter().any(|segment| segment.self_replacements.iter()
            .any(|branch| reads_activation_history(&branch.condition)))
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
    let readers: Vec<_> = definition.abilities.iter().map(ability_reads_activation_history).collect();
    if !readers.iter().any(|reader| *reader) { return Ok(()); }
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
    Ok(())
}

//! Compiler-authored trigger occurrences, independent of their runtime words.
use crate::ability::AbilityKind;
use crate::cards::{CardDefinition, builders::CardTextError};
use sha2::{Digest, Sha256};

/// Run after every root-program finalizer. The complete typed definition graph
/// defines the authored namespace; each root slot is a distinct occurrence,
/// including repeated identical printed abilities. Card allocator ids and
/// printing metadata are not part of that namespace.
///
/// Nested quoted triggers are deliberately unstamped in this increment. Their
/// own lowering owner must supply a typed authored occurrence path before text
/// changing readers can admit them. A parent trigger's stamp never proves a
/// nested trigger's identity.
pub(super) fn stamp_trigger_definitions(definition: &mut CardDefinition) -> Result<(), CardTextError> {
    stamp_trigger_definitions_with_generated(definition, &[])
}

pub(super) fn stamp_trigger_definitions_with_generated(
    definition: &mut CardDefinition,
    generated: &[ironsmith_core::LinkedExileDefinition],
) -> Result<(), CardTextError> {
    if !definition.abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))) {
        return Ok(());
    }
    let Some(bytes) = authored_root_namespace_with_generated(definition, generated)? else {
        // Older opaque pair/child stamps can contain local allocations. Do
        // not turn that unavailable canonical proof into a new stable stamp.
        // The programs and their existing link owners still execute normally.
        for ability in &mut definition.abilities {
            if let AbilityKind::Triggered(triggered) = &mut ability.kind {
                triggered.effects.trigger_definition = None;
            }
        }
        return Ok(());
    };
    for (occurrence, ability) in definition.abilities.iter_mut().enumerate() {
        if let AbilityKind::Triggered(triggered) = &mut ability.kind {
            let mut digest = Sha256::new();
            digest.update(b"ironsmith-trigger-definition-v2\0");
            digest.update(&bytes);
            digest.update((occurrence as u64).to_le_bytes());
            triggered.effects = std::mem::take(&mut triggered.effects)
                .with_trigger_definition(ironsmith_core::LinkedExileDefinition(digest.finalize().into()));
        }
    }
    Ok(())
}

/// Shared normalization for activation and trigger definition finalizers.
pub(super) fn authored_root_namespace(definition: &CardDefinition) -> Result<Option<Vec<u8>>, CardTextError> {
    authored_root_namespace_with_generated(definition, &[])
}

pub(super) fn authored_root_namespace_with_generated(
    definition: &CardDefinition,
    generated: &[ironsmith_core::LinkedExileDefinition],
) -> Result<Option<Vec<u8>>, CardTextError> {
    let mut authored = definition.clone();
    authored.canonical_text.clear();
    authored.ability_labels.clear();
    authored.card.other_face = None;
    authored.card.first_printed_set_name = None;
    for ability in &mut authored.abilities {
        let program = match &mut ability.kind {
            AbilityKind::Triggered(triggered) => {
                triggered.presentation_label = None;
                &mut triggered.effects
            },
            AbilityKind::Activated(activated) => &mut activated.effects,
            AbilityKind::Static(_) => continue,
        };
        // Generated identities are outputs of this namespace, never inputs.
        *program = std::mem::take(program)
            .with_trigger_definition(ironsmith_core::LinkedExileDefinition([0; 32]));
        program.trigger_definition = None;
        program.activation_definition = None;
    }
    // The transport-owned typed graph walker recognizes actual CardId
    // newtypes, including nested token/emblem/copy definitions and references.
    // Converting once to its wire vocabulary avoids remapping a typed compiler
    // payload both before and after its erased effect envelope is decoded.
    let wire = ironsmith_compiled_artifact::wire_definition_from_serializable(&authored)
        .map_err(|error| CardTextError::InvariantViolation(format!("cannot retain authored definition graph: {error}")))?;
    let mut ids = std::collections::BTreeMap::from([(authored.card.id.0, 0u32)]);
    let (normalized, retained_definition) = ironsmith_artifact_effect_decoder::authored_definition_graph(&wire, &mut |id| {
        if let Some(mapped) = ids.get(&id) { return Ok(*mapped); }
        let ordinal = u32::try_from(ids.len()).map_err(|_| "authored definition graph exceeds CardId vocabulary".to_string())?;
        ids.insert(id, ordinal);
        Ok(ordinal)
    }, generated).map_err(|error| CardTextError::InvariantViolation(format!("cannot normalize authored definition graph: {error}")))?;
    if retained_definition { return Ok(None); }
    serde_json::to_vec(&normalized).map(Some).map_err(|error|
        CardTextError::InvariantViolation(format!("cannot retain authored ability definition identity: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body(id: u32, name: &str, amount: i32) -> CardDefinition {
        let ability = crate::ability::Ability::triggered(
            crate::triggers::Trigger::this_dies(),
            vec![crate::effect::Effect::gain_life(amount)],
        );
        crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(id), name)
            .card_types(vec![ironsmith_core::CardType::Creature])
            .with_ability(ability.clone()).with_ability(ability).build()
    }
    fn stamps(definition: &CardDefinition) -> Vec<ironsmith_core::LinkedExileDefinition> {
        definition.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.effects.retained_trigger_definition(),
            _ => None,
        }).collect()
    }
    #[test]
    fn authored_occurrences_are_repeatable_and_exclude_local_card_ids() {
        let mut first = body(100, "Printed trigger", 1);
        let mut allocated_again = body(200, "Printed trigger", 1);
        stamp_trigger_definitions(&mut first).unwrap();
        stamp_trigger_definitions(&mut allocated_again).unwrap();
        assert_eq!(stamps(&first), stamps(&allocated_again));
        assert_eq!(stamps(&first).len(), 2);
        assert_ne!(stamps(&first)[0], stamps(&first)[1], "equal printed lines remain distinct occurrences");
        let before = stamps(&first);
        stamp_trigger_definitions(&mut first).unwrap();
        assert_eq!(stamps(&first), before);
    }
    #[test]
    fn complete_typed_face_and_body_define_the_namespace() {
        let mut first = body(100, "Front face", 1);
        let mut back = body(100, "Back face", 1);
        let mut other_program = body(100, "Front face", 2);
        for definition in [&mut first, &mut back, &mut other_program] {
            stamp_trigger_definitions(definition).unwrap();
        }
        assert_ne!(stamps(&first), stamps(&back));
        assert_ne!(stamps(&first), stamps(&other_program));
    }
    #[test]
    fn compiler_authors_the_complete_composed_root_and_refinalizes_repeatably() {
        let mut definition = body(100, "Composed trigger", 1);
        let AbilityKind::Triggered(triggered) = &mut definition.abilities[0].kind else { unreachable!() };
        triggered.effects = std::mem::take(&mut triggered.effects)
            .with_trigger_definition(ironsmith_core::LinkedExileDefinition([1; 32]));
        triggered.effects.extend(crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::gain_life(2)])
            .with_trigger_definition(ironsmith_core::LinkedExileDefinition([2; 32])));
        assert!(triggered.effects.retained_trigger_definition().is_none());
        stamp_trigger_definitions(&mut definition).unwrap();
        let first = stamps(&definition);
        assert_eq!(first.len(), 2, "complete final compiler root replaces composition loss with authored proof");
        stamp_trigger_definitions(&mut definition).unwrap();
        assert_eq!(stamps(&definition), first);
    }
    #[test]
    fn activation_and_trigger_finalizers_share_a_stable_authored_namespace() {
        let mut definition = body(100, "Mixed printed face", 1);
        definition.abilities.push(crate::ability::Ability::activated(crate::cost::TotalCost::free(), vec![
            crate::effect::Effect::conditional(crate::effect::Condition::ThisAbilityActivatedThisTurnAtLeast(4),
                vec![crate::effect::Effect::gain_life(2)], Vec::new()),
        ]));
        let stamp_both = |definition: &mut CardDefinition| {
            super::super::activation_definitions::stamp_activation_definitions(definition).unwrap();
            stamp_trigger_definitions(definition).unwrap();
        };
        stamp_both(&mut definition);
        let first = serde_json::to_value(&definition).unwrap();
        stamp_both(&mut definition);
        assert_eq!(serde_json::to_value(&definition).unwrap(), first);
        stamp_trigger_definitions(&mut definition).unwrap();
        super::super::activation_definitions::stamp_activation_definitions(&mut definition).unwrap();
        assert_eq!(serde_json::to_value(&definition).unwrap(), first, "generated stamps cannot feed back into either definition owner");
    }

    #[test]
    fn nested_token_card_allocations_do_not_change_trigger_or_activation_namespace() {
        let make = |root: u32, token: u32| {
            let token = crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(token), "Spirit")
                .token().card_types(vec![ironsmith_core::CardType::Creature])
                .subtypes(vec![ironsmith_core::Subtype::Spirit]).build();
            crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(root), "Token trigger")
                .card_types(vec![ironsmith_core::CardType::Creature])
                .with_ability(crate::ability::Ability::triggered(crate::triggers::Trigger::this_dies(),
                    vec![crate::effect::Effect::create_tokens(token, 1)])).build()
        };
        let mut first = make(100, 200);
        let mut allocated_again = make(300, 400);
        assert_eq!(authored_root_namespace(&first).unwrap(), authored_root_namespace(&allocated_again).unwrap());
        stamp_trigger_definitions(&mut first).unwrap();
        stamp_trigger_definitions(&mut allocated_again).unwrap();
        assert_eq!(stamps(&first), stamps(&allocated_again));
        let original_tokens = first.abilities.iter().filter_map(|ability| {
            let AbilityKind::Triggered(triggered) = &ability.kind else { return None; };
            triggered.effects[0].as_create_token().map(|effect| effect.token.card.id)
        }).collect::<Vec<_>>();
        assert_eq!(original_tokens, vec![ironsmith_core::CardId::from_raw(200)], "normalization does not mutate executable definitions");
    }

    #[test]
    fn definition_graph_normalization_preserves_card_reference_aliases_and_ordinary_numbers() {
        let make = |first: u32, second: u32, linked: u32, amount: i32| {
            let mut token = crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(first), "Linked token")
                .token().card_types(vec![ironsmith_core::CardType::Creature]).build();
            token.card.other_face = Some(ironsmith_core::CardId::from_raw(linked));
            let other = crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(second), "Other token")
                .token().card_types(vec![ironsmith_core::CardType::Creature]).build();
            crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(99), "Graph witness")
                .with_ability(crate::ability::Ability::triggered(crate::triggers::Trigger::this_dies(), vec![
                    crate::effect::Effect::create_tokens(token, 1),
                    crate::effect::Effect::create_tokens(other, 1),
                    crate::effect::Effect::gain_life(amount),
                ])).build()
        };
        let shared = authored_root_namespace(&make(101, 102, 102, 7)).unwrap();
        assert_eq!(shared, authored_root_namespace(&make(901, 902, 902, 7)).unwrap());
        assert_ne!(shared, authored_root_namespace(&make(101, 102, 101, 7)).unwrap(), "different graph edges remain different");
        assert_ne!(shared, authored_root_namespace(&make(101, 102, 102, 8)).unwrap(), "ordinary numeric values are not CardIds");
    }

    #[test]
    fn nested_references_to_the_authored_root_keep_the_same_graph_identity() {
        let make = |root: u32, token: u32, reference: u32| {
            let mut nested = crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(token), "Root reference")
                .token().card_types(vec![ironsmith_core::CardType::Creature]).build();
            nested.card.other_face = Some(ironsmith_core::CardId::from_raw(reference));
            crate::cards::builders::CardDefinitionBuilder::new(ironsmith_core::CardId::from_raw(root), "Root graph")
                .with_ability(crate::ability::Ability::triggered(crate::triggers::Trigger::this_dies(),
                    vec![crate::effect::Effect::create_tokens(nested, 1)])).build()
        };
        let root_edge = authored_root_namespace(&make(0, 2, 0)).unwrap();
        assert_eq!(root_edge, authored_root_namespace(&make(10, 20, 10)).unwrap());
        assert_ne!(root_edge, authored_root_namespace(&make(10, 20, 30)).unwrap());
    }

    #[test]
    fn rendering_the_root_does_not_change_its_authored_namespace() {
        let mut definition = body(100, "Root presentation", 1);
        let expected = authored_root_namespace(&definition).unwrap();
        definition.canonical_text = "Rendered instructions only".into();
        definition.ability_labels = vec!["Display line one".into(), "Display line two".into()];
        if let AbilityKind::Triggered(triggered) = &mut definition.abilities[0].kind {
            triggered.presentation_label = Some(crate::ability::PresentationLabel::AbilityWord("Rendered trigger label".into()));
        }
        assert_eq!(authored_root_namespace(&definition).unwrap(), expected);
    }

    #[test]
    fn opaque_link_identity_keeps_legacy_activation_owner_without_inventing_trigger_proof() {
        let mut definition = body(100, "Independent native pair", 1);
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([37; 32]), pair: 2,
        };
        if let AbilityKind::Triggered(triggered) = &mut definition.abilities[0].kind {
            triggered.effects.linked_exile_pair = Some(pair);
        }
        definition.abilities.push(crate::ability::Ability::activated(crate::cost::TotalCost::free(), vec![
            crate::effect::Effect::conditional(crate::effect::Condition::ThisAbilityActivatedThisTurnAtLeast(4),
                vec![crate::effect::Effect::gain_life(2)], Vec::new()),
        ]));
        assert!(authored_root_namespace(&definition).unwrap().is_none());
        let mut face = definition.card.clone();
        face.id = ironsmith_core::CardId::from_raw(0);
        face.other_face = None;
        face.first_printed_set_name = None;
        let mut digest = Sha256::new();
        digest.update(b"ironsmith-activated-definition-v1\0");
        digest.update(serde_json::to_vec(&(face, &definition.abilities)).unwrap());
        digest.update(2u64.to_le_bytes());
        let legacy = ironsmith_core::LinkedExileDefinition(digest.finalize().into());
        super::super::activation_definitions::stamp_activation_definitions(&mut definition).unwrap();
        stamp_trigger_definitions(&mut definition).unwrap();
        let AbilityKind::Activated(activated) = &definition.abilities[2].kind else { unreachable!() };
        assert_eq!(activated.effects.activation_definition, Some(legacy), "the existing history reader retains its exact original owner");
        assert!(stamps(&definition).is_empty(), "an opaque pair does not become new canonical trigger proof");
        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else { unreachable!() };
        assert_eq!(triggered.effects.linked_exile_pair, Some(pair), "the native link remains immutable");
    }

}

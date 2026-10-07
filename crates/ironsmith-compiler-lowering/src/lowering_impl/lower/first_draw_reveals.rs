//! Finalize front-end-authored reveal links without inferring a link from names,
//! labels, source-object identity or proximity of independently parsed abilities.
use crate::ability::AbilityKind;
use crate::cards::{CardDefinition, builders::CardTextError};
use sha2::{Digest, Sha256};

/// Evidence emitted only by this compiler construction owner. Runtime/native
/// programs never acquire it merely by carrying an equal-looking pair.
pub(super) struct GeneratedFirstDrawDefinition(ironsmith_core::LinkedExileDefinition);
impl GeneratedFirstDrawDefinition {
    pub(super) fn definition(&self) -> ironsmith_core::LinkedExileDefinition { self.0 }
}

pub(super) fn stamp_first_draw_pairs(definition: &mut CardDefinition) -> Result<Option<GeneratedFirstDrawDefinition>, CardTextError> {
    stamp_first_draw_pairs_with_generated(definition, &[])
}

/// Validate only the front-end-owned first-draw construction groups. These
/// definition inputs can be normalized alongside an independent numeric group.
pub(super) fn compiler_pair_definitions(definition: &CardDefinition) -> Result<Vec<ironsmith_core::LinkedExileDefinition>, CardTextError> {
    let mut producers = std::collections::BTreeMap::<u32, usize>::new();
    let mut consumers = std::collections::BTreeMap::<u32, usize>::new();
    let mut generated = Vec::new();
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Static(ability) => if let ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn { linked_reveal_pair: Some(pair), .. } = &ability.payload {
                *producers.entry(pair.pair).or_default() += 1;
                if !generated.contains(&pair.definition) { generated.push(pair.definition); }
            },
            AbilityKind::Triggered(ability) => if let ironsmith_core::TriggerKind::PlayerRevealsCard { first_draw_pair: Some(pair), .. } = &ability.trigger.kind {
                *consumers.entry(pair.pair).or_default() += 1;
                if !generated.contains(&pair.definition) { generated.push(pair.definition); }
            },
            _ => {}
        }
    }
    if producers.is_empty() && consumers.is_empty() { return Ok(Vec::new()); }
    if producers.keys().ne(consumers.keys()) || producers.values().any(|count| *count != 1) {
        return Err(CardTextError::InvariantViolation("first-draw authored group lost its producer or linked triggers".into()));
    }
    Ok(generated)
}

pub(super) fn stamp_first_draw_pairs_with_generated(
    definition: &mut CardDefinition,
    other_generated: &[ironsmith_core::LinkedExileDefinition],
) -> Result<Option<GeneratedFirstDrawDefinition>, CardTextError> {
    let mut generated = compiler_pair_definitions(definition)?;
    if generated.is_empty() { return Ok(None); }
    for identity in other_generated {
        if !generated.contains(identity) { generated.push(*identity); }
    }
    let bytes = super::trigger_definitions::authored_root_namespace_with_generated(definition, &generated)?
        .ok_or_else(|| CardTextError::InvariantViolation("first-draw definition contains an unproven opaque identity namespace".into()))?;
    let mut digest = Sha256::new();
    digest.update(b"ironsmith-first-draw-definition-v2\0");
    digest.update(bytes);
    let stamp = ironsmith_core::LinkedExileDefinition(digest.finalize().into());
    for ability in &mut definition.abilities {
        match &mut ability.kind {
            AbilityKind::Static(ability) => if let ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn { linked_reveal_pair: Some(pair), .. } = &mut ability.payload {
                pair.definition = stamp;
            },
            AbilityKind::Triggered(ability) => if let ironsmith_core::TriggerKind::PlayerRevealsCard { first_draw_pair: Some(pair), .. } = &mut ability.trigger.kind {
                pair.definition = stamp;
            },
            _ => {}
        }
    }
    Ok(Some(GeneratedFirstDrawDefinition(stamp)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith_core::{CardId, CardType, LinkedExileDefinition, LinkedExilePair, StaticAbilityPayload, Subtype, TriggerKind};

    fn token_pair(root_id: u32, token_id: u32) -> CardDefinition {
        let placeholder = LinkedExilePair { definition: LinkedExileDefinition([0; 32]), pair: 7 };
        let mut reveal = crate::static_abilities::StaticAbility::reveal_first_card_you_draw_each_turn(true, false);
        let StaticAbilityPayload::RevealFirstCardYouDrawEachTurn { linked_reveal_pair, .. } = &mut reveal.payload else { unreachable!() };
        *linked_reveal_pair = Some(placeholder);
        let mut trigger = crate::triggers::Trigger::this_dies();
        trigger.kind = TriggerKind::PlayerRevealsCard {
            player: crate::target::PlayerFilter::You,
            filter: crate::target::ObjectFilter::default(),
            from_source: true,
            first_draw_pair: Some(placeholder),
        };
        let token = crate::cards::builders::CardDefinitionBuilder::new(CardId::from_raw(token_id), "Named Demon")
            .token().card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Demon]).build();
        crate::cards::builders::CardDefinitionBuilder::new(CardId::from_raw(root_id), "First draw token source")
            .with_ability(crate::ability::Ability::static_ability(reveal))
            .with_ability(crate::ability::Ability::triggered(trigger,
                vec![crate::effect::Effect::create_tokens(token, 1)])).build()
    }

    fn finalize(definition: &mut CardDefinition) -> LinkedExileDefinition {
        let proof = stamp_first_draw_pairs(definition).unwrap().unwrap();
        super::super::activation_definitions::stamp_activation_definitions_with_generated(definition, &[proof.definition()]).unwrap();
        super::super::trigger_definitions::stamp_trigger_definitions_with_generated(definition, &[proof.definition()]).unwrap();
        proof.definition()
    }

    #[test]
    fn first_draw_token_pair_and_trigger_stamps_exclude_nested_allocations_and_generated_feedback() {
        let mut first = token_pair(101, 201);
        let mut reallocated = token_pair(701, 901);
        let expected = finalize(&mut first);
        assert_eq!(finalize(&mut reallocated), expected);
        let AbilityKind::Triggered(first_trigger) = &first.abilities[1].kind else { unreachable!() };
        let AbilityKind::Triggered(second_trigger) = &reallocated.abilities[1].kind else { unreachable!() };
        assert!(first_trigger.effects.retained_trigger_definition().is_some());
        assert_eq!(first_trigger.effects.retained_trigger_definition(), second_trigger.effects.retained_trigger_definition());
        assert_eq!(first_trigger.effects[0].as_create_token().unwrap().token.card.id, CardId::from_raw(201));
        let TriggerKind::PlayerRevealsCard { first_draw_pair: Some(pair), .. } = &first_trigger.trigger.kind else { unreachable!() };
        assert_eq!(*pair, LinkedExilePair { definition: expected, pair: 7 });
        let before = serde_json::to_value(&first).unwrap();
        assert_eq!(finalize(&mut first), expected);
        assert_eq!(serde_json::to_value(&first).unwrap(), before, "compiler re-finalization never feeds generated hashes back into the namespace");
    }

    #[test]
    fn independent_opaque_identity_is_not_normalized_as_a_compiler_first_draw_pair() {
        let mut definition = token_pair(101, 201);
        let AbilityKind::Triggered(triggered) = &mut definition.abilities[1].kind else { unreachable!() };
        let foreign = LinkedExilePair { definition: LinkedExileDefinition([73; 32]), pair: 2 };
        triggered.effects.linked_exile_pair = Some(foreign);
        assert!(stamp_first_draw_pairs(&mut definition).is_err());
        let AbilityKind::Triggered(triggered) = &definition.abilities[1].kind else { unreachable!() };
        assert_eq!(triggered.effects.linked_exile_pair, Some(foreign));
        assert!(triggered.effects.retained_trigger_definition().is_none());
    }
    fn combined_pairs(root_id: u32, token_id: u32) -> CardDefinition {
        let mut definition = token_pair(root_id, token_id);
        let number = crate::effect::Value::SourceChosenNumber { if_unset: Some(0), pair: None };
        definition.abilities.push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::as_enters_effect_program(
                vec![crate::effect::Effect::new(crate::effects::ChooseNumberEffect {
                    chooser: crate::target::PlayerFilter::You, min: 0, max: Some(7), source_owned: true,
                })].into(), "this creature", false, false, None,
            ),
        ));
        definition.abilities.push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::characteristic_defining_pt(number.clone(), number),
        ));
        definition.abilities.push(crate::ability::Ability::activated(crate::cost::TotalCost::free(), vec![
            crate::effect::Effect::conditional(crate::effect::Condition::ThisAbilityActivatedThisTurnAtLeast(4),
                vec![crate::effect::Effect::gain_life(2)], Vec::new()),
        ]));
        definition
    }

    fn finalize_combined(definition: &mut CardDefinition) -> (LinkedExileDefinition, LinkedExileDefinition) {
        let numeric = super::super::source_numbers::bind_source_number_pair(definition).unwrap().unwrap();
        let numeric = numeric.definition();
        let reveal = stamp_first_draw_pairs_with_generated(definition, &[numeric]).unwrap().unwrap().definition();
        let generated = [numeric, reveal];
        super::super::activation_definitions::stamp_activation_definitions_with_generated(definition, &generated).unwrap();
        super::super::trigger_definitions::stamp_trigger_definitions_with_generated(definition, &generated).unwrap();
        (numeric, reveal)
    }

    #[test]
    fn numeric_first_draw_and_root_owners_compose_without_allocator_or_generated_stamp_feedback() {
        let mut first = combined_pairs(100, 200);
        let mut reallocated = combined_pairs(900, 700);
        let owners = finalize_combined(&mut first);
        assert_eq!(finalize_combined(&mut reallocated), owners);
        assert_ne!(owners.0, owners.1, "independent numeric and reveal groups retain distinct namespaces");
        let AbilityKind::Triggered(triggered) = &first.abilities[1].kind else { unreachable!() };
        let AbilityKind::Triggered(other_trigger) = &reallocated.abilities[1].kind else { unreachable!() };
        assert!(triggered.effects.retained_trigger_definition().is_some());
        assert_eq!(triggered.effects.retained_trigger_definition(), other_trigger.effects.retained_trigger_definition());
        assert_eq!(triggered.effects[0].as_create_token().unwrap().token.card.id, CardId::from_raw(200));
        let AbilityKind::Activated(activated) = &first.abilities[4].kind else { unreachable!() };
        let AbilityKind::Activated(other_activation) = &reallocated.abilities[4].kind else { unreachable!() };
        assert!(activated.effects.activation_definition.is_some());
        assert_eq!(activated.effects.activation_definition, other_activation.effects.activation_definition);
        let AbilityKind::Static(entry) = &first.abilities[2].kind else { unreachable!() };
        let StaticAbilityPayload::AsEntersEffectProgram { program, .. } = &entry.payload else { unreachable!() };
        assert_eq!(program.source_number_pair, Some(LinkedExilePair { definition: owners.0, pair: 2 }));
        let AbilityKind::Static(cda) = &first.abilities[3].kind else { unreachable!() };
        let StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } = &cda.payload else { unreachable!() };
        let expected = crate::effect::Value::SourceChosenNumber { if_unset: Some(0), pair: program.source_number_pair };
        assert_eq!(power, &expected);
        assert_eq!(toughness, &expected);
        let before = serde_json::to_value(&first).unwrap();
        assert_eq!(finalize_combined(&mut first), owners);
        assert_eq!(serde_json::to_value(&first).unwrap(), before);
    }

    #[test]
    fn numeric_construction_does_not_clear_an_unrelated_opaque_owner() {
        let mut definition = combined_pairs(100, 200);
        let foreign = LinkedExilePair { definition: LinkedExileDefinition([81; 32]), pair: 9 };
        let AbilityKind::Triggered(triggered) = &mut definition.abilities[1].kind else { unreachable!() };
        triggered.effects.linked_exile_pair = Some(foreign);
        assert!(super::super::source_numbers::bind_source_number_pair(&mut definition).is_err());
        let AbilityKind::Triggered(triggered) = &definition.abilities[1].kind else { unreachable!() };
        assert_eq!(triggered.effects.linked_exile_pair, Some(foreign));
        assert!(triggered.effects.retained_trigger_definition().is_none());
    }

    #[test]
    fn token_word_roles_join_the_combined_namespace_without_rewriting_native_ids() {
        use ironsmith_core::{TokenNameTextRole, TokenTextRoles};
        let make = |root, token, role: Option<TokenNameTextRole>| {
            let mut definition = combined_pairs(root, token);
            let AbilityKind::Triggered(triggered) = &mut definition.abilities[1].kind else { unreachable!() };
            triggered.effects = triggered.effects.clone().try_map_effects(|effect| {
                let mut create = effect.as_create_token().unwrap().clone();
                create.token.card.name = "Demon Token".into();
                create.text_roles = role.map(|role| TokenTextRoles::authored(role, 0));
                Ok::<_, ()>(crate::effect::Effect::new(create))
            }).unwrap();
            definition
        };
        let mut owners_by_role = Vec::new();
        for role in [None, Some(TokenNameTextRole::Explicit), Some(TokenNameTextRole::SubtypeDerived)] {
            let mut first = make(100, 200, role);
            let mut reallocated = make(900, 700, role);
            let owners = finalize_combined(&mut first);
            assert_eq!(finalize_combined(&mut reallocated), owners);
            assert_ne!(owners.0, owners.1);
            let AbilityKind::Triggered(triggered) = &first.abilities[1].kind else { unreachable!() };
            let AbilityKind::Triggered(other) = &reallocated.abilities[1].kind else { unreachable!() };
            assert_eq!(triggered.effects.retained_trigger_definition(), other.effects.retained_trigger_definition());
            assert!(triggered.effects.retained_trigger_definition().is_some());
            let create = triggered.effects[0].as_create_token().unwrap();
            assert_eq!(create.token.card.id, CardId::from_raw(200));
            assert_eq!(create.text_roles.as_ref().map(|roles| roles.name), role);
            let AbilityKind::Activated(activated) = &first.abilities[4].kind else { unreachable!() };
            let AbilityKind::Activated(other) = &reallocated.abilities[4].kind else { unreachable!() };
            assert_eq!(activated.effects.activation_definition, other.effects.activation_definition);
            assert!(activated.effects.activation_definition.is_some());
            let before = serde_json::to_value(&first).unwrap();
            assert_eq!(finalize_combined(&mut first), owners);
            assert_eq!(serde_json::to_value(&first).unwrap(), before);
            owners_by_role.push(owners);
        }
        assert_ne!(owners_by_role[0], owners_by_role[1], "missing roles are unknown, not explicit-name proof");
        assert_ne!(owners_by_role[1], owners_by_role[2], "equal token names do not erase their distinct authored roles");
    }

}

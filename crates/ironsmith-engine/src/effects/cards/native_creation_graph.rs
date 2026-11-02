//! Closed lexical creation graphs, checked directly over native owned programs.
//! This visitor never serializes or reconstructs an executable program.
use super::{CreateCardsEffect, definition_scope::CardDefinitionScopeEffect};
use crate::{CardDefinition, Effect};
use ironsmith_core::CardId;
use std::collections::HashSet;

type Environment = Vec<(CardId, HashSet<CardId>)>;

pub(crate) fn validate_native_definition(definition: &CardDefinition) -> Result<(), String> {
    definition_in(definition, &Vec::new(), None, 0)
}

pub(crate) fn validate_native_scope_nodes(
    scope: CardId,
    nodes: &[CardDefinition],
) -> Result<(), String> {
    admit(scope, nodes, 0).map(|_| ())
}

fn admit(scope: CardId, nodes: &[CardDefinition], depth: usize) -> Result<HashSet<CardId>, String> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for node in nodes {
        if node.card.is_token || node.card.name.trim().is_empty() {
            return Err("real-card catalog requires named non-token definitions".into());
        }
        if !ids.insert(node.card.id) || !names.insert(node.card.name.trim().to_lowercase()) {
            return Err("real-card catalog has duplicate identity or name".into());
        }
    }
    if !ids.contains(&scope) {
        return Err("real-card lexical scope has no owning definition".into());
    }
    let environment = vec![(scope, ids.clone())];
    for node in nodes {
        definition_in(node, &environment, Some(scope), depth + 1)?;
    }
    Ok(ids)
}

fn depth_guard(depth: usize) -> Result<(), String> {
    if depth > 256 {
        Err("native real-card graph exceeds the structural nesting limit".into())
    } else {
        Ok(())
    }
}

fn effect_in(
    effect: &Effect,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<(), String> {
    depth_guard(depth)?;
    if let Some(wrapper) = effect.downcast_ref::<CardDefinitionScopeEffect>() {
        if shallow.is_some() {
            return Err("real-card catalog members must remain shallow lexical references".into());
        }
        let ids = admit(wrapper.scope, &wrapper.definitions, depth + 1)?;
        let mut local = environment.clone();
        local.push((wrapper.scope, ids));
        return effect_in(&wrapper.effect, &local, None, depth + 1);
    }
    if let Some(create) = effect.downcast_ref::<CreateCardsEffect>() {
        let refs: HashSet<_> = create.definition_refs.iter().copied().collect();
        if refs.len() != create.definition_refs.len() {
            return Err("duplicate identity in real-card reference subset".into());
        }
        if let Some(expected) = shallow {
            if create.definition_scope != Some(expected) || !create.definitions.is_empty() {
                return Err(
                    "real-card catalog members must use their own shallow lexical scope".into(),
                );
            }
        }
        match create.definition_scope {
            Some(scope) => {
                let ids = if create.definitions.is_empty() {
                    environment
                        .iter()
                        .rev()
                        .find(|(id, _)| *id == scope)
                        .map(|(_, ids)| ids.clone())
                        .ok_or_else(|| "real-card reference has no lexical catalog".to_string())?
                } else {
                    admit(scope, &create.definitions, depth + 1)?
                };
                if !refs.is_subset(&ids) {
                    return Err("real-card reference escapes its lexical catalog".into());
                }
            }
            None => {
                if !refs.is_empty() {
                    return Err("real-card references require an explicit lexical scope".into());
                }
                // A legacy inline catalog is a closed declaration, not an import
                // from whichever lexical program happens to instantiate it.
                for node in &create.definitions {
                    definition_in(node, &Vec::new(), None, depth + 1)?;
                }
            }
        }
        return Ok(());
    }
    // These program owners retain abilities or entry modifications, whose
    // native static wrappers require checked canonical traversal as well as
    // ordinary child-effect visitation.
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>() {
        ability_in(grant.ability.clone(), environment, shallow, depth + 1)?;
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        for modification in continuous
            .modification
            .iter()
            .chain(&continuous.additional_modifications)
        {
            modification_in(modification.clone(), environment, shallow, depth + 1)?;
        }
        for modification in &continuous.runtime_modifications {
            if let crate::effects::continuous::RuntimeModification::CopyOfWithAbilities {
                abilities,
                ..
            } = modification
            {
                for ability in abilities {
                    ability_in(ability.clone(), environment, shallow, depth + 1)?;
                }
            }
        }
    }
    if let Some(movement) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        for modification in &movement.entry_modifications {
            modification_in(modification.clone(), environment, shallow, depth + 1)?;
        }
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantObjectAbilityEffect>() {
        ability_in(grant.ability.clone(), environment, shallow, depth + 1)?;
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantEffect>() {
        grant.grantable.clone().try_map(
            |value| static_in(value, environment, shallow, depth + 1),
            |value| {
                effect_in(&value, environment, shallow, depth + 1)?;
                Ok(value)
            },
            |value| cost_in(value, environment, shallow, depth + 1),
        )?;
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantBySpecEffect>() {
        grant.spec.clone().try_map(
            |value| static_in(value, environment, shallow, depth + 1),
            |value| {
                effect_in(&value, environment, shallow, depth + 1)?;
                Ok(value)
            },
            |value| cost_in(value, environment, shallow, depth + 1),
        )?;
    }
    if let Some(delayed) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        if let Some(prepayment) = &delayed.prepayment {
            prepayment
                .cost
                .clone()
                .try_map(|value| cost_in(value, environment, shallow, depth + 1))?;
        }
    }
    if let Some(emblem) = effect.downcast_ref::<crate::effects::CreateEmblemEffect>() {
        for ability in &emblem.emblem.abilities {
            ability_in(ability.clone(), environment, shallow, depth + 1)?;
        }
    }
    if !trusted_native_program_effect(effect) {
        return Err("opaque native executor has no complete graph ownership visitor".into());
    }
    let mut result = Ok(());
    effect.visit_child_effects(&mut |child| {
        if result.is_ok() {
            result = effect_in(child, environment, shallow, depth + 1);
        }
    });
    effect.visit_card_definitions(&mut |definition| {
        if result.is_ok() {
            result = definition_in(definition, environment, shallow, depth + 1);
        }
    });
    result
}

fn trusted_native_program_effect(effect: &Effect) -> bool {
    if super::trusted_native_scalar_effect(effect) {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::GrantObjectAbilityEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::GrantEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::GrantBySpecEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CreateEmblemEffect>()
        .is_some()
    {
        return true;
    }
    // These concrete owners implement native child/definition visitation;
    // recognizing a new opaque implementation requires an explicit addition.
    if effect
        .downcast_ref::<crate::effects::AddColorlessManaEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::BindXValueEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ChangeTextEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ChooseModeEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CollectManaPaymentsEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CopyFromDefinitionEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::CumulativeUpkeepEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::DestroyEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::DiscardEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::DoubleCountersEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::FirstResolutionEachTurnEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachAnnouncedTargetSlotEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachObjectCorrelatedResultEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>()
        .is_some()
    {
        return true;
    }
    if effect.downcast_ref::<crate::effects::IfEffect>().is_some() {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::InvestigateEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::LoseLifeEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ManaRestrictedEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ManaRetainedEffect>()
        .is_some()
    {
        return true;
    }
    if effect.downcast_ref::<crate::effects::MayEffect>().is_some() {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::NamedActionProgramEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::NoteLifeTotalEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::PreventAllDamageEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::PreventDamageEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::RedirectNextTimeDamageToSourceEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::ReflexiveTriggerEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::RepeatEffectsEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::RepeatProcessEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::RepeatProcessPromptEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::SacrificeEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::UnlessActionEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::UnlessPaysEffect>()
        .is_some()
    {
        return true;
    }
    if effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .is_some()
    {
        return true;
    }
    false
}

fn cost_in(
    cost: crate::costs::Cost,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<crate::costs::Cost, String> {
    let model = cost
        .compiled_model()
        .ok_or_else(|| "native cost has no complete typed model for graph admission".to_string())?;
    model.clone().try_map_effect(|effect| {
        effect_in(&effect, environment, shallow, depth + 1)?;
        Ok::<_, String>(effect)
    })?;
    Ok(cost)
}

fn static_in(
    ability: crate::static_abilities::StaticAbility,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<crate::static_abilities::StaticAbility, String> {
    let model = ability.canonical_model().ok_or_else(|| {
        "native static ability has no complete typed model for graph admission".to_string()
    })?;
    model.try_map(
        Ok::<_, String>,
        |effect| {
            effect_in(&effect, environment, shallow, depth + 1)?;
            Ok(effect)
        },
        |cost| cost_in(cost, environment, shallow, depth + 1),
        Ok::<_, String>,
    )?;
    Ok(ability)
}

fn ability_in(
    ability: crate::Ability,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<crate::Ability, String> {
    ability.try_map(
        |ability| static_in(ability, environment, shallow, depth + 1),
        Ok::<_, String>,
        |effect| {
            effect_in(&effect, environment, shallow, depth + 1)?;
            Ok(effect)
        },
        |cost| cost_in(cost, environment, shallow, depth + 1),
        Ok::<_, String>,
    )
}
fn copy_in(
    values: crate::snapshot::CopiableValues,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<crate::snapshot::CopiableValues, String> {
    depth_guard(depth)?;
    if values.spell_effect.is_unavailable() {
        return Err(
            "frozen copy has no complete spell-program evidence for graph admission".into(),
        );
    }
    let retained: crate::snapshot::RetainedCopiableValues<
        crate::Ability,
        Effect,
        crate::costs::Cost,
    > = values.into();
    let retained = retained
        .try_map_abilities(|value| ability_in(value, environment, shallow, depth + 1))?
        .try_map_effects(|value| {
            effect_in(&value, environment, shallow, depth + 1)?;
            Ok::<_, String>(value)
        })?
        .try_map_costs(|value| cost_in(value, environment, shallow, depth + 1))?;
    Ok(retained.into())
}
fn modification_in(
    modification: crate::continuous::Modification,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<(), String> {
    modification.try_map_payloads(
        |ability| static_in(ability, environment, shallow, depth + 1),
        |ability| ability_in(ability, environment, shallow, depth + 1),
        |copy| copy_in(copy, environment, shallow, depth + 1),
        |text| {
            for ability in text.abilities.iter() {
                ability_in(ability.clone(), environment, shallow, depth + 1)?;
            }
            Ok(text)
        },
        |value| {
            crate::continuous::RetainedRestriction::from(value.clone())
                .try_map_ability(|ability| static_in(ability, environment, shallow, depth + 1))
                .map(|_| value)
        },
        |value| {
            crate::object::RetainedAuraAttachmentMetadata::from(value.clone())
                .try_map_ability(|ability| static_in(ability, environment, shallow, depth + 1))
                .map(|_| value)
        },
    )?;
    Ok(())
}

fn definition_in(
    definition: &CardDefinition,
    environment: &Environment,
    shallow: Option<CardId>,
    depth: usize,
) -> Result<(), String> {
    depth_guard(depth)?;
    definition.clone().try_map(
        |ability| {
            ability.try_map(
                |ability| static_in(ability, environment, shallow, depth + 1),
                Ok::<_, String>,
                |effect| {
                    effect_in(&effect, environment, shallow, depth + 1)?;
                    Ok(effect)
                },
                |cost| cost_in(cost, environment, shallow, depth + 1),
                Ok::<_, String>,
            )
        },
        |effect| {
            effect_in(&effect, environment, shallow, depth + 1)?;
            Ok(effect)
        },
        |cost| cost_in(cost, environment, shallow, depth + 1),
        |method| {
            method.try_map(
                |effect| {
                    effect_in(&effect, environment, shallow, depth + 1)?;
                    Ok(effect)
                },
                |cost| cost_in(cost, environment, shallow, depth + 1),
            )
        },
        |cost| cost.try_map(|cost| cost_in(cost, environment, shallow, depth + 1)),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sequence(effects: Vec<Effect>) -> Effect {
        Effect::new(crate::effects::SequenceEffect::new(effects))
    }
    use crate::CardDefinitionBuilder;
    fn creation(scope: CardId, refs: Vec<CardId>) -> Effect {
        Effect::new(CreateCardsEffect {
            definitions: vec![],
            definition_scope: Some(scope),
            definition_refs: refs,
            operation_label: None,
            selection: ironsmith_core::CardCreationSelection::Named("Native Beta".into()),
            count: 1.into(),
            player: ironsmith_core::PlayerFilter::You,
            zone: crate::Zone::Hand,
            library_position: ironsmith_core::CardCreationLibraryPosition::Top,
        })
    }
    fn catalog() -> Vec<CardDefinition> {
        let a = CardId::new();
        let b = CardId::new();
        vec![
            CardDefinitionBuilder::new(a, "Native Alpha")
                .with_spell_effect(vec![creation(a, vec![b])])
                .build(),
            CardDefinitionBuilder::new(b, "Native Beta")
                .with_spell_effect(vec![creation(a, vec![a])])
                .build(),
        ]
    }
    #[test]
    fn native_mutual_catalog_is_finite_and_requires_lexical_binding() {
        let nodes = catalog();
        let scope = nodes[0].card.id;
        validate_native_scope_nodes(scope, &nodes).unwrap();
        assert!(
            validate_native_definition(&nodes[0]).is_err(),
            "shallow imports require their owning scope"
        );
        let bound = super::super::definition_scope::bind_card_creation_scope_runtime(
            nodes[0].clone(),
            scope,
            &nodes,
        )
        .unwrap();
        validate_native_definition(&bound).unwrap();
    }
    #[test]
    fn native_catalog_rejects_dangling_ids_tokens_duplicates_and_nested_closed_graphs() {
        let nodes = catalog();
        let scope = nodes[0].card.id;
        let mut bad = nodes.clone();
        bad[0].spell_effect = Some(vec![creation(scope, vec![CardId::new()])].into());
        assert!(validate_native_scope_nodes(scope, &bad).is_err());
        let mut bad = nodes.clone();
        bad[1].card.id = scope;
        assert!(validate_native_scope_nodes(scope, &bad).is_err());
        let mut bad = nodes.clone();
        bad[1].card.name = " native alpha ".into();
        assert!(validate_native_scope_nodes(scope, &bad).is_err());
        let mut bad = nodes.clone();
        bad[1].card.is_token = true;
        assert!(validate_native_scope_nodes(scope, &bad).is_err());
        let closed = super::super::definition_scope::bind_card_creation_scope_runtime(
            nodes[0].clone(),
            scope,
            &nodes,
        )
        .unwrap();
        let mut bad = nodes.clone();
        bad[0] = closed;
        assert!(validate_native_scope_nodes(scope, &bad).is_err());
    }
    #[test]
    fn native_scope_shadowing_is_lexical_and_does_not_leak_to_siblings() {
        let nodes = catalog();
        let scope = nodes[0].card.id;
        let mut foreign = nodes.clone();
        foreign[0].card.name = "Foreign Alpha".into();
        foreign[1].card.name = "Foreign Beta".into();
        let foreign_id = CardId::new();
        foreign[1].card.id = foreign_id;
        foreign[0].spell_effect = Some(vec![creation(scope, vec![foreign_id])].into());
        let outer = Effect::new(CardDefinitionScopeEffect {
            scope,
            definitions: nodes.clone(),
            effect: Box::new(sequence(vec![
                Effect::new(CardDefinitionScopeEffect {
                    scope,
                    definitions: foreign,
                    effect: Box::new(creation(scope, vec![foreign_id])),
                }),
                creation(scope, vec![nodes[1].card.id]),
            ])),
        });
        effect_in(&outer, &Vec::new(), None, 0).unwrap();
        let leaked = Effect::new(CardDefinitionScopeEffect {
            scope,
            definitions: nodes,
            effect: Box::new(creation(scope, vec![foreign_id])),
        });
        assert!(effect_in(&leaked, &Vec::new(), None, 0).is_err());
    }
}

#[cfg(test)]
mod opaque_ownership_test {
    use super::*;
    #[derive(Debug, Clone)]
    struct HiddenProgram(Effect);
    impl crate::effects::EffectExecutor for HiddenProgram {
        fn execute(
            &self,
            _: &mut crate::GameState,
            _: &mut crate::effects::ExecutionContext,
        ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
            let _ = &self.0;
            Ok(crate::effect::EffectOutcome::resolved())
        }
    }
    #[test]
    fn opaque_native_program_cannot_hide_a_dangling_creation_reference() {
        let hidden = Effect::new(HiddenProgram(Effect::new(CreateCardsEffect {
            definitions: vec![],
            definition_scope: Some(CardId::new()),
            definition_refs: vec![CardId::new()],
            operation_label: None,
            selection: ironsmith_core::CardCreationSelection::Named("Unbound".into()),
            count: 1.into(),
            player: ironsmith_core::PlayerFilter::You,
            zone: crate::Zone::Hand,
            library_position: ironsmith_core::CardCreationLibraryPosition::Top,
        })));
        assert!(
            effect_in(&hidden, &Vec::new(), None, 0)
                .unwrap_err()
                .contains("opaque native executor")
        );
    }
    #[test]
    fn direct_native_legacy_creation_admission_rejects_hidden_or_unbound_programs() {
        use super::super::create_cards::CardCreationDefinitionGraphExt;
        let scope = CardId::new();
        let dangling = Effect::new(CreateCardsEffect {
            definitions: vec![],
            definition_scope: Some(scope),
            definition_refs: vec![scope],
            operation_label: None,
            selection: ironsmith_core::CardCreationSelection::Named("Unbound".into()),
            count: 1.into(),
            player: ironsmith_core::PlayerFilter::You,
            zone: crate::Zone::Hand,
            library_position: ironsmith_core::CardCreationLibraryPosition::Top,
        });
        for effect in [dangling.clone(), Effect::new(HiddenProgram(dangling))] {
            let definition = crate::CardDefinitionBuilder::new(CardId::new(), "Legacy input")
                .with_spell_effect(vec![effect])
                .build();
            let direct = CreateCardsEffect {
                definitions: vec![definition],
                definition_scope: None,
                definition_refs: vec![],
                operation_label: None,
                selection: ironsmith_core::CardCreationSelection::Named("Legacy input".into()),
                count: 1.into(),
                player: ironsmith_core::PlayerFilter::You,
                zone: crate::Zone::Hand,
                library_position: ironsmith_core::CardCreationLibraryPosition::Top,
            };
            assert!(
                direct.admitted_definitions().is_err(),
                "public native creation must admit a closed definition independently of an outer compiler wrapper"
            );
        }
    }
}

#[cfg(test)]
mod native_static_ownership_tests {
    use super::*;
    #[test]
    fn canonical_only_native_static_children_are_visited_and_checked_inside_a_grant() {
        let scope = CardId::new();
        let replacement = Effect::new(CreateCardsEffect {
            definitions: vec![],
            definition_scope: Some(scope),
            definition_refs: vec![scope],
            operation_label: None,
            selection: ironsmith_core::CardCreationSelection::Named("Unbound".into()),
            count: 1.into(),
            player: ironsmith_core::PlayerFilter::You,
            zone: crate::Zone::Hand,
            library_position: ironsmith_core::CardCreationLibraryPosition::Top,
        });
        let rule = crate::static_abilities::StaticAbility::new(
            crate::static_abilities::CardCreationSelectionReplacement {
                operation_label: Some("make".into()),
                player: ironsmith_core::PlayerFilter::Any,
                replacement,
                display: "selection rule".into(),
            },
        );
        assert!(rule.compiled_model().is_none());
        assert!(rule.canonical_model().is_some());
        let mut children = 0;
        crate::ability::visit_static_owned_effects(&rule, &mut |_| children += 1);
        assert_eq!(
            children, 1,
            "native canonical models own executable children even without retained compiler models"
        );
        let grant = Effect::new(crate::effects::GrantNextSpellAbilityEffect::new(
            ironsmith_core::PlayerFilter::You,
            crate::target::ObjectFilter::default(),
            crate::Ability::static_ability(rule),
        ));
        assert!(
            effect_in(&grant, &Vec::new(), None, 0).is_err(),
            "a known grant cannot hide an unbound native-static replacement graph"
        );
    }
}

#[cfg(test)]
mod copied_program_ownership_tests {
    use super::*;
    #[derive(Debug, Clone)]
    struct OpaqueStatic;
    impl crate::static_abilities::StaticAbilityKind for OpaqueStatic {
        fn id(&self) -> crate::static_abilities::StaticAbilityId {
            crate::static_abilities::StaticAbilityId::Flying
        }
        fn display(&self) -> String {
            "unrepresented native static".into()
        }
    }
    #[test]
    fn frozen_copied_abilities_and_counter_definition_dependencies_require_typed_proof() {
        let definition = crate::CardDefinitionBuilder::new(CardId::new(), "Copied source").build();
        let mut game = crate::GameState::new(vec!["A".into(), "B".into()], 20);
        let id = game.create_object_from_definition(
            &definition,
            crate::PlayerId(0),
            crate::Zone::Battlefield,
        );
        let values = crate::snapshot::CopiableValues::from_object(game.object(id).unwrap());
        assert!(copy_in(values.clone(), &Vec::new(), None, 0).is_ok());
        let opaque = crate::Ability::static_ability(crate::static_abilities::StaticAbility::new(
            OpaqueStatic,
        ));
        let mut bad = values.clone();
        bad.abilities = std::sync::Arc::new(vec![opaque.clone()]);
        assert!(copy_in(bad, &Vec::new(), None, 0).is_err());
        let mut bad = values;
        bad.definition_dependencies = Some(crate::snapshot::CopyDefinitionDependencies {
            ability_counter_definitions: std::collections::BTreeMap::from([(
                "grant".into(),
                vec![opaque],
            )]),
            custom_subtypes: vec![],
        });
        assert!(copy_in(bad, &Vec::new(), None, 0).is_err());
    }
}

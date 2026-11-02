//! Copiable token grants captured by an explicitly named operation boundary.
use crate::ability::{Ability, AbilityKind};
use crate::effects::ExecutionError;
use crate::filter::PlayerFilterExt;
use crate::snapshot::{CopiableValues, CopyDefinitionDependencies};
use crate::static_abilities::StaticAbilityModelInterpreter;
use crate::{GameState, PlayerId};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct NamedActionTokenGrant {
    pub abilities: Vec<Ability>,
    pub dependencies: CopyDefinitionDependencies<Ability>,
}

pub(crate) fn capture_named_action_token_grants(game: &GameState, label: &str, player: PlayerId)
    -> Result<Vec<NamedActionTokenGrant>, ExecutionError>
{
    let mut grants = Vec::new();
    let effects = game.try_all_continuous_effects_arc().map_err(ExecutionError::ContinuousDiscovery)?;
    for object in game.objects_in_deterministic_order().into_iter().filter(|object| !game.is_phased_out(object.id)) {
        if !matches!(object.zone, crate::Zone::Battlefield | crate::Zone::Command)
            && !object.abilities.iter().any(|ability| ability.functions_in(&object.zone)
                && matches!(&ability.kind, AbilityKind::Static(ability) if
                    ability.compiled_model().is_some_and(|model| matches!(&model.payload,
                        ironsmith_core::StaticAbilityPayload::NamedActionTokenAbilityGrant { .. }))))
        { continue; }
        let source = crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics_and_effects(object, game, &effects)?;
        for ability in source.abilities.iter().filter(|ability| ability.functions_in(&source.zone)) {
            let AbilityKind::Static(ability) = &ability.kind else { continue; };
            let Some(model) = ability.compiled_model() else { continue; };
            let ironsmith_core::StaticAbilityPayload::NamedActionTokenAbilityGrant {
                label: rule_label, player: actor, abilities, ..
            } = &model.payload else { continue; };
            if !rule_label.eq_ignore_ascii_case(label)
                || !actor.matches_player(player, &game.filter_context_for(source.controller, Some(source.object_id)))
            { continue; }
            let dependencies = source.copiable_values.definition_dependencies.clone().ok_or_else(||
                ExecutionError::IncompleteEvidence("token grant lacks its source definition dependencies".into()))?;
            grants.push(NamedActionTokenGrant {
                abilities: abilities.iter().map(StaticAbilityModelInterpreter::ability_from_model).collect(),
                dependencies,
            });
        }
    }
    Ok(grants)
}

/// Called by each creation original after physical groups commit, before
/// deferred entry/creation additions. No bubbled child-event inference.
pub(crate) fn capture_named_action_created_objects(game: &GameState, ctx: &mut crate::effects::ExecutionContext,
    created: &[crate::ObjectId]) -> Result<(), ExecutionError>
{
    let Some(packet) = &mut ctx.named_action_created_objects else { return Ok(()); };
    for id in created {
        let object = game.object(*id).ok_or_else(|| ExecutionError::IncompleteEvidence("created token original lost its exact object before capture".into()))?;
        let snapshot = crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)?;
        if !packet.iter().any(|prior| prior.object_id == snapshot.object_id) { packet.push(snapshot); }
    }
    let captured = packet.clone();
    ctx.set_tagged_objects("__named_action_created_objects", captured);
    Ok(())
}

fn include_dependencies(target: &mut CopyDefinitionDependencies<Ability>, source: &CopyDefinitionDependencies<Ability>) {
    for (name, program) in &source.ability_counter_definitions {
        target.ability_counter_definitions.entry(name.clone()).or_insert_with(|| program.clone());
    }
    for subtype in &source.custom_subtypes {
        ironsmith_core::custom_subtypes::merge_subtype_associations(&mut target.custom_subtypes, *subtype);
    }
}

pub(crate) fn apply_named_action_grants_to_definition(definition: &mut crate::CardDefinition, grants: &[NamedActionTokenGrant]) {
    if !grants.is_empty() {
        definition.ability_labels = crate::runtime_display::definition_ability_labels(definition);
    }
    for grant in grants {
        definition.ability_labels.extend(grant.abilities.iter().map(crate::runtime_display::ability_surface_text));
        definition.abilities.extend(grant.abilities.iter().cloned());
        for (name, program) in &grant.dependencies.ability_counter_definitions {
            definition.ability_counter_definitions.entry(name.clone()).or_insert_with(|| program.clone());
        }
        for subtype in &grant.dependencies.custom_subtypes {
            ironsmith_core::custom_subtypes::merge_subtype_associations(&mut definition.card.custom_subtypes, *subtype);
        }
    }
}

fn grant_copiable_face(values: &mut CopiableValues, grants: &[NamedActionTokenGrant]) {
    for grant in grants {
        Arc::make_mut(&mut values.abilities).extend(grant.abilities.iter().cloned());
        match &mut values.definition_dependencies {
            Some(dependencies) => include_dependencies(dependencies, &grant.dependencies),
            None => values.definition_dependencies = Some(grant.dependencies.clone()),
        }
    }
    if let Some(other) = &mut values.other_copy_face { grant_copiable_face(other, grants); }
}

pub(crate) fn apply_named_action_grants_to_object(object: &mut crate::object::Object, grants: &[NamedActionTokenGrant]) {
    if grants.is_empty() { return; }
    let mut other = object.copied_face_values.as_ref().and_then(|values| values.other_copy_face.clone());
    for grant in grants {
        object.include_source_definition_dependencies(&grant.dependencies);
        for ability in &grant.abilities {
            let mut text = object.compiled_card_text.to_string();
            crate::static_abilities::add_copy_exception_ability(&mut object.abilities,
                &mut object.ability_labels, &mut text, ability.clone(), None);
            object.compiled_card_text = text.into();
        }
    }
    if object.copied_face_values.is_some() {
        if let Some(other) = &mut other { grant_copiable_face(other, grants); }
        let mut values = CopiableValues::from_object(object);
        for grant in grants {
            match &mut values.definition_dependencies {
                Some(dependencies) => include_dependencies(dependencies, &grant.dependencies),
                None => values.definition_dependencies = Some(grant.dependencies.clone()),
            }
        }
        values.other_copy_face = other;
        object.copied_face_values = Some(Box::new(values));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::effects::{CreateTokenEffect, EffectContext, NamedActionProgramEffect};
    use crate::static_abilities::{CompiledStaticAbility, StaticAbility};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::{CardId, Effect, Zone};
    const A: PlayerId = PlayerId(0);
    fn token(name: &str) -> crate::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name).token()
            .card_types(vec![crate::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build()
    }
    fn source() -> crate::CardDefinition {
        let grant = CompiledStaticAbility::named_action_token_ability_grant("forge bundle", PlayerFilter::You,
            vec![ironsmith_core::Ability::triggered(
                crate::triggers::Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
                vec![Effect::gain_life(5)])], "An operation-scoped combat grant");
        let mut definition = CardDefinitionBuilder::new(CardId::new(), "Operation grant source")
            .card_types(vec![crate::CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::from_model(grant))).build();
        definition.card.custom_subtypes.push(crate::Subtype::Custom(
            ironsmith_core::custom_subtypes::CustomSubtype::new("Scope vocabulary", ironsmith_core::types::SubtypeFamily::Creature).unwrap()));
        definition.ability_counter_definitions.insert("scope flying".into(), vec![Ability::static_ability(StaticAbility::flying())]);
        definition
    }
    fn operation(body: Vec<Effect>) -> Effect {
        Effect::new(NamedActionProgramEffect {
            label: "forge bundle".into(), player: PlayerFilter::You, objects: Some(ChooseSpec::Tagged("__named_action_created_objects".into())),
            completion: ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            effects: body,
        })
    }
    fn triggered_count(game: &GameState, object: crate::ObjectId) -> usize {
        game.current_characteristics(object).unwrap().abilities.iter()
            .filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count()
    }
    #[test]
    fn original_tokens_gain_copiable_abilities_dependencies_and_outer_scope_restores() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&source(), A, Zone::Battlefield);
        let effect = operation(vec![Effect::new(CreateTokenEffect::new(token("Original bundle"), 2, PlayerFilter::You))]);
        let mut ctx = EffectContext::new_default(source, A);
        crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(ctx.named_action_token_scope.is_none());
        assert!(ctx.named_action_created_objects.is_none());
        let completed = ctx.get_tagged_all("__named_action_created_objects").unwrap();
        assert_eq!(completed.len(), 2);
        let originals = game.battlefield.iter().copied().filter(|id| *id != source).collect::<Vec<_>>();
        assert_eq!(originals.len(), 2);
        for id in &originals {
            assert_eq!(triggered_count(&game, *id), 1);
            let copied = game.calculated_copiable_values(*id).unwrap();
            assert_eq!(copied.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count(), 1);
            let dependencies = copied.definition_dependencies.unwrap();
            assert!(dependencies.ability_counter_definitions.contains_key("scope flying"));
            assert!(dependencies.custom_subtypes.iter().any(|subtype| subtype.to_string()=="Scope vocabulary"));
        }
        crate::effects::execute_effect(&mut game, &Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::SpecificObject(originals[0]), 1, PlayerFilter::You)), &mut ctx).unwrap();
        let copied = *game.battlefield.iter().find(|id| **id != source && !originals.contains(id)).unwrap();
        assert_eq!(triggered_count(&game, copied), 1);
        let definition = game.object(copied).unwrap().to_card_definition();
        assert!(definition.ability_counter_definitions.contains_key("scope flying"));
        crate::effects::execute_effect(&mut game, &Effect::new(CreateTokenEffect::new(token("Outside bundle"), 1, PlayerFilter::You)), &mut ctx).unwrap();
        let outsider = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name.as_ref()=="Outside bundle").unwrap();
        assert_eq!(triggered_count(&game, outsider), 0);
        let event = crate::triggers::TriggerEvent::new_with_provenance(crate::events::DamageEvent::with_cause(
            originals[0], crate::events::DamageTarget::Player(PlayerId(1)), 1, true,
            crate::events::EventCause::effect()), crate::provenance::ProvNodeId::default());
        assert_eq!(crate::triggers::check_triggers(&game, &event).iter().filter(|trigger| trigger.source==originals[0]).count(),1);
    }
    #[test]
    fn count_multiplier_inherits_grant_but_independent_added_program_does_not() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut definition=source();
        definition.abilities.push(Ability::static_ability(StaticAbility::double_token_creation_replacement(PlayerFilter::You,"Double originals".into())));
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
            source,A,crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(PlayerFilter::You),
            crate::replacement::ReplacementAction::Additionally(vec![Effect::new(CreateTokenEffect::new(token("Independent addition"),1,PlayerFilter::You))])));
        let mut ctx=EffectContext::new_default(source,A);
        crate::effects::execute_effect(&mut game,&operation(vec![Effect::new(CreateTokenEffect::new(token("Original bundle"),1,PlayerFilter::You))]),&mut ctx).unwrap();
        let originals=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Original bundle").collect::<Vec<_>>();
        let additions=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Independent addition").collect::<Vec<_>>();
        assert_eq!(originals.len(),2);
        let packet=ctx.get_tagged_all("__named_action_created_objects").unwrap();
        assert_eq!(packet.len(),2);
        assert!(packet.iter().all(|snapshot|originals.contains(&snapshot.object_id)));
        assert!(!additions.is_empty());
        assert!(originals.iter().all(|id|triggered_count(&game,*id)==1));
        assert!(additions.iter().all(|id|triggered_count(&game,*id)==0));
    }
    #[test]
    fn provider_is_sampled_when_each_token_original_is_proposed_not_when_operation_begins() {
        for entering in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let performer_definition = CardDefinitionBuilder::new(CardId::new(), "Independent performer")
                .card_types(vec![crate::CardType::Artifact]).build();
            let performer = game.create_object_from_definition(&performer_definition, A, Zone::Battlefield);
            let provider = game.create_object_from_definition(&source(), A,
                if entering { Zone::Hand } else { Zone::Battlefield });
            let change = Effect::new(crate::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(provider),
                if entering { Zone::Battlefield } else { Zone::Graveyard }, false));
            let create = Effect::new(CreateTokenEffect::new(token("Live provider bundle"), 1, PlayerFilter::You));
            let mut ctx = EffectContext::new_default(performer, A);
            crate::effects::execute_effect(&mut game, &operation(vec![change, create]), &mut ctx).unwrap();
            let created = *game.battlefield.iter().find(|id|
                game.object(**id).unwrap().name.as_ref() == "Live provider bundle").unwrap();
            assert_eq!(triggered_count(&game, created), usize::from(entering),
                "the creation sees the provider after the earlier payment/program instructions");
            assert!(ctx.named_action_token_scope.is_none());
            assert_eq!(ctx.get_tagged_all("__named_action_created_objects").unwrap().len(), 1);
        }
    }
    #[test]
    fn a_prepared_creation_keeps_its_own_grant_and_definition_evidence() {
        use crate::effects::EffectExecutor;
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let provider = game.create_object_from_definition(&source(), A, Zone::Battlefield);
        let performer = game.create_object_from_definition(&token("Performer"), A, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(performer, A);
        ctx.named_action_token_scope = Some(("forge bundle".into(), A));
        let effect = CreateTokenEffect::new(token("Prepared bundle"), 1, PlayerFilter::You);
        let proposal = effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap();
        game.move_object(provider, Zone::Graveyard, crate::events::EventCause::effect()).unwrap();
        proposal.commit_original_with_outputs(&mut game, &mut ctx).unwrap();
        let created = *game.battlefield.iter().find(|id|
            game.object(**id).unwrap().name.as_ref() == "Prepared bundle").unwrap();
        assert_eq!(triggered_count(&game, created), 1);
        assert!(game.calculated_copiable_values(created).unwrap().definition_dependencies.unwrap()
            .ability_counter_definitions.contains_key("scope flying"));
    }

}

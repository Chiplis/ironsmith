//! Typed authored-word traversal of complete static definitions.
//!
//! Models are cloned before any edit. Presentation, names, reference/link
//! identities, literal mana symbols and rules implied by keyword abilities
//! survive unchanged. A model with opaque word provenance or a lossy native
//! materialization path is a checked hold, even when the selected word is absent.
//! This owner never compares runtime costs/abilities through rendered equality.

use super::text_change_predicates::{
    rewrite_anthem_count_words, rewrite_condition_words, rewrite_filter_words,
    rewrite_player_filter_words, rewrite_value_words,
};
use super::text_change_programs::{
    rewrite_ability_words, rewrite_cost_words, rewrite_program_words,
    rewrite_total_cost_words, rewrite_triggered_ability_words,
};
use super::text_changes::{
    TextChangeDomainError as Error, rewrite_attachment_words, rewrite_core_landwalk_words,
    rewrite_protection_words,
};
use crate::costs::Cost;
use crate::effect::Effect;
use crate::static_abilities::{CompiledStaticAbility, StaticAbilityId};
use crate::triggers::Trigger;
use ironsmith_core::{
    AnthemValue, Condition, ObjectFilter, PlayerFilter, StaticAbilityPayload as P,
    Subtype, TextChange, ThisSpellCostCondition, Value,
};

type AbilityModel = ironsmith_core::Ability<CompiledStaticAbility, Trigger, Effect, Cost>;

fn hold(model: &CompiledStaticAbility) -> Error {
    Error::StaticAbility(model.id.unwrap_or(StaticAbilityId::RuleFallbackText))
}

fn optional_filter(filter: &mut Option<ObjectFilter>, change: TextChange) -> Result<(), Error> {
    if let Some(filter) = filter { *filter = rewrite_filter_words(filter, change)?; }
    Ok(())
}

fn optional_player(player: &mut Option<PlayerFilter>, change: TextChange) -> Result<(), Error> {
    if let Some(player) = player { *player = rewrite_player_filter_words(player, change)?; }
    Ok(())
}

fn optional_condition(condition: &mut Option<Condition>, change: TextChange) -> Result<(), Error> {
    if let Some(condition) = condition { *condition = rewrite_condition_words(condition, change)?; }
    Ok(())
}

fn optional_value(value: &mut Option<Value>, change: TextChange) -> Result<(), Error> {
    if let Some(value) = value { *value = rewrite_value_words(value, change)?; }
    Ok(())
}

fn subtype_words(subtypes: &mut Vec<Subtype>, change: TextChange) {
    change.replace_subtype_words(subtypes);
}

fn effects_words(effects: &mut Vec<Effect>, change: TextChange) -> Result<(), Error> {
    *effects = effects.iter().map(|effect| effect.with_text_change(change)).collect::<Result<_, _>>()?;
    Ok(())
}

fn statics_words(abilities: &mut Vec<CompiledStaticAbility>, change: TextChange) -> Result<(), Error> {
    *abilities = abilities.iter().map(|ability| rewrite_static_model_words(ability, change))
        .collect::<Result<_, _>>()?;
    Ok(())
}

fn ability_model_words(ability: &mut AbilityModel, change: TextChange) -> Result<(), Error> {
    match &mut ability.kind {
        ironsmith_core::AbilityKind::Static(model) => *model = rewrite_static_model_words(model, change)?,
        ironsmith_core::AbilityKind::Triggered(model) => *model = rewrite_triggered_ability_words(model, change)?,
        ironsmith_core::AbilityKind::Activated(model) => {
            // The activated vocabulary is already identical on both sides.
            // Use its complete owner rather than reconstructing only cost/effects.
            let runtime = crate::ability::Ability {
                kind: crate::ability::AbilityKind::Activated(model.clone()),
                functional_zones: ability.functional_zones.clone(),
            };
            let transformed = rewrite_ability_words(&runtime, change)?;
            let crate::ability::AbilityKind::Activated(rewritten) = transformed.kind else {
                return Err(Error::ActivatedAbility);
            };
            *model = rewritten;
        }
    }
    Ok(())
}

fn abilities_words(abilities: &mut [AbilityModel], change: TextChange) -> Result<(), Error> {
    for ability in abilities { ability_model_words(ability, change)?; }
    Ok(())
}

fn anthem_value_words(value: &mut AnthemValue, change: TextChange) -> Result<(), Error> {
    match value {
        AnthemValue::Fixed(_) => {}
        AnthemValue::Dynamic(value) => *value = rewrite_value_words(value, change)?,
        AnthemValue::PerCount { count, .. } | AnthemValue::CappedPerCount { count, .. } => {
            *count = rewrite_anthem_count_words(count, change)?;
        }
    }
    Ok(())
}

/// The returned model keeps the original id/label and every presentation field.
/// The immutable runtime wrapper owns occurrence identity and memoization.
/// Do not compare this result with the source using `PartialEq`: nested runtime
/// costs can define equality through their rendered text.
pub(crate) fn rewrite_static_model_words(
    model: &CompiledStaticAbility,
    change: TextChange,
) -> Result<CompiledStaticAbility, Error> {
    let mut rewritten = model.clone();
    match &mut rewritten.payload {
        P::None | P::SelfSubjectSurface { .. } => {
            if !wordless_leaf(model.id) { return Err(hold(model)); }
        }
        P::SourceLineKeywordGroup { .. } | P::SourceLineStaticGroup { .. } => {}
        P::LookAtSourceExiledCards { pair: _, source: _ } => {},
        P::Protection(from) => *from = rewrite_protection_words(from, change)?,
        P::Landwalk(kind) => *kind = rewrite_core_landwalk_words(*kind, change),
        P::Enchant(filter) => *filter = rewrite_attachment_words(filter, change)?,
        P::HexproofFrom(filter) | P::BandsWithOther(filter)
        | P::PreventAllCombatDamageToPermanentsMatching(filter)
        | P::PreventAllCombatDamageToAndByPermanentsMatching(filter)
        | P::PreventAllNoncombatDamageToPermanentsMatching(filter)
        | P::PreventAllDamageToPermanentsMatching(filter)
        | P::FlashIfTargetsMatching(filter) | P::RemoveAllAbilities(filter)
        | P::RemoveAllAbilitiesExceptMana(filter) | P::MakeColorless(filter)
        | P::EntersTappedForFilter(filter) | P::EntersUntappedForFilter(filter)
        | P::GoadMatching { filter } | P::CostIncreaseLife { filter, .. }
        | P::ActivateAbilitiesAsThoughHaste { filter, .. }
        | P::LoyaltyAbilitiesAnyTime { filter }
        | P::UntapDuringEachOtherPlayersUntapStep { filter, .. }
        | P::SetName { filter, .. } | P::AddSupertypes { filter, .. }
        | P::RemoveSupertypes { filter, .. } | P::AddChosenCreatureType { filter, .. }
        | P::AddChosenBasicLandType { filter, .. } | P::AddChosenColor { filter, .. }
        | P::SetChosenColor { filter, .. } | P::SetBasePowerToughness { filter, .. }
        | P::SetBasePower { filter, .. } | P::SetBaseToughness { filter, .. }
        | P::AddCardTypes { filter, .. }
        | P::SetCardTypes { filter, .. } | P::AddAllSubtypesOfFamily { filter, .. }
        | P::RevealFromHandAsEnters { filter, .. } | P::RedirectZoneChange { filter, .. }
        | P::DrawReplacementRevealTopMatchingToHandRestBottom { filter, .. }
        | P::DiscardOrRedirectReplacement { filter, .. }
        | P::SacrificeOrRedirectReplacement { filter, .. }
        | P::RevealCardOrEnterTapped { filter, .. } | P::RedirectWouldEnter { filter, .. }
        | P::CanBlockAdditionalForEach { filter, .. }
        | P::GrantSpellKeyword { filter, .. }
        | P::AlternativeCastFromZoneForFilter { filter, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        P::ConditionalAttackRequirement { trigger, required } => {
            *trigger = rewrite_filter_words(trigger, change)?;
            *required = rewrite_filter_words(required, change)?;
        }
        P::LegendRuleDoesntApplyToController { filter } => {
            // This id selects a hardcoded token filter in native materialization.
            if model.id == Some(StaticAbilityId::LegendRuleDoesntApplyToControllerTokens) {
                return Err(hold(model));
            }
            *filter = rewrite_filter_words(filter, change)?;
        }
        P::Companion(condition) => {
            if let ironsmith_core::CompanionDeckCondition::CreatureSubtypes(subtypes) = condition {
                subtype_words(subtypes, change);
            }
        }
        P::Anthem(anthem) => {
            optional_filter(&mut anthem.filter, change)?;
            anthem_value_words(&mut anthem.power, change)?;
            anthem_value_words(&mut anthem.toughness, change)?;
            optional_condition(&mut anthem.condition, change)?;
        }
        P::AttachedAbilityGrant(grant) => {
            ability_model_words(&mut grant.ability, change)?;
            abilities_words(&mut grant.additional_abilities, change)?;
            optional_condition(&mut grant.condition, change)?;
        }
        P::GrantObjectAbilityForFilter(grant) => {
            grant.filter = rewrite_filter_words(&grant.filter, change)?;
            ability_model_words(&mut grant.ability, change)?;
            abilities_words(&mut grant.additional_abilities, change)?;
            optional_condition(&mut grant.condition, change)?;
        }
        P::Conditional { ability, condition } => {
            **ability = rewrite_static_model_words(ability, change)?;
            *condition = rewrite_condition_words(condition, change)?;
        }
        P::CopyActivatedAbilities(copy) => copy.filter = rewrite_filter_words(&copy.filter, change)?,
        P::CopyStaticAbilityVariants(copy) => copy.filter = rewrite_filter_words(&copy.filter, change)?,
        P::CopyTriggeredAbilities(copy) => copy.filter = rewrite_filter_words(&copy.filter, change)?,
        P::CostReduction(spec) => {
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            spec.amount = rewrite_value_words(&spec.amount, change)?;
            optional_condition(&mut spec.condition, change)?;
            if let Some(intersection) = &mut spec.characteristic_intersection {
                intersection.comparison = rewrite_filter_words(&intersection.comparison, change)?;
            }
        }
        P::CostIncrease(spec) => {
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            spec.amount = rewrite_value_words(&spec.amount, change)?;
            optional_condition(&mut spec.condition, change)?;
        }
        P::CostReductionManaCost(spec) => {
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            optional_condition(&mut spec.condition, change)?;
        }
        P::CostIncreaseManaCost(spec) => {
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            optional_condition(&mut spec.condition, change)?;
        }
        P::ThisSpellCostReduction(spec) => {
            spec.amount = rewrite_value_words(&spec.amount, change)?;
            spell_cost_condition_words(&mut spec.condition, change)?;
            optional_filter(&mut spec.affinity_filter, change)?;
        }
        P::ThisSpellCostReductionManaCost(spec) => {
            optional_value(&mut spec.repetitions, change)?;
            spell_cost_condition_words(&mut spec.condition, change)?;
        }
        P::ThisSpellCastRestriction { kind, .. } => {
            // Historical label-only kinds still infer executable predicates
            // from strings. Only the newer closed timing enum is complete.
            if kind.timing.is_none() { return Err(hold(model)); }
        }
        P::ThisSpellXMaximum { maximum, .. } => *maximum = rewrite_value_words(maximum, change)?,
        P::ThisSpellXMinimum { minimum, .. } => *minimum = rewrite_value_words(minimum, change)?,
        P::DieRollResultAdjustment(spec) => {
            if (spec.reroll && (spec.life_cost != 0 || spec.amount != 0 || spec.mana_cost.is_none()))
                || (!spec.reroll && spec.mana_cost.is_some()) { return Err(hold(model)); }
            spec.player = rewrite_player_filter_words(&spec.player, change)?;
        }
        P::LevelAbility(level) => statics_words(&mut level.abilities, change)?,
        P::EquipmentGrant(abilities) => statics_words(abilities, change)?,
        P::SoulbondSharedAbility(ability) => **ability = rewrite_static_model_words(ability, change)?,
        P::SoulbondSharedObjectAbility(ability) => ability_model_words(ability, change)?,
        P::RemoveAbilityForFilter { filter, ability, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            **ability = rewrite_static_model_words(ability, change)?;
        }
        P::RemoveObjectAbilitiesForFilter { filter, abilities, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            abilities_words(abilities, change)?;
        }
        P::PreventAllDamageToSelfFromSourcesMatching(spec) => {
            spec.source_filter = rewrite_filter_words(&spec.source_filter, change)?;
        }
        P::RuleRestriction { restriction, additional_restrictions, .. } => {
            restriction_words(restriction, change)?;
            for restriction in additional_restrictions { restriction_words(restriction, change)?; }
        }
        P::PregameAction { effects, .. } => effects_words(effects, change)?,
        P::Ward(cost) | P::Morph(cost) | P::Disguise(cost) | P::Megamorph(cost) => {
            *cost = rewrite_total_cost_words(cost, change)?;
        }
        P::Splice(spec) => spec.cost = rewrite_total_cost_words(&spec.cost, change)?,
        P::Escalate(spec) => spec.cost = rewrite_total_cost_words(&spec.cost, change)?,
        P::CanBlockAsThoughReachForSubtype(subtype) => change.replace_subtype_word(subtype),
        P::TargetingAsThoughNoAbility(spec) => {
            optional_filter(&mut spec.objects, change)?;
            optional_player(&mut spec.players, change)?;
            spec.sources_controlled_by = rewrite_player_filter_words(&spec.sources_controlled_by, change)?;
        }
        P::BlockingAsThoughNoLandwalk(spec) => {
            spec.objects = rewrite_filter_words(&spec.objects, change)?;
            if let Some(kind) = &mut spec.landwalk { *kind = rewrite_core_landwalk_words(*kind, change); }
        }
        P::CantAttackUnlessCondition { condition, .. } => attack_condition_words(condition, change)?,
        P::AttackCost { attackers, cost, .. } => {
            *attackers = rewrite_filter_words(attackers, change)?;
            *cost = rewrite_total_cost_words(cost, change)?;
        }
        P::BlockCost { blockers, attackers, cost, .. } => {
            *blockers = rewrite_filter_words(blockers, change)?;
            *attackers = rewrite_filter_words(attackers, change)?;
            *cost = rewrite_total_cost_words(cost, change)?;
        }
        P::UntapStepLimit { player, filter, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        P::PlayerProtectionFrom { player, source_filter, .. }
        | P::RedirectDamageToSourceController { target_player_filter: player, source_filter, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
            *source_filter = rewrite_filter_words(source_filter, change)?;
        }
        P::SetColors { filter, colors, .. } | P::AddColors { filter, colors } => {
            // ALL also represents the phrase "all colors", without five
            // authored words. The payload has no spelling provenance.
            if colors.count() == 5 { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            change.replace_color_words(colors);
        }
        P::SetMaximumHandSize { player, .. } | P::ReduceMaximumHandSize { player, .. }
        | P::IncreaseMaximumHandSize { player, .. }
        | P::MaximumHandSizeSevenMinusYourGraveyardCardTypes { player, .. }
        | P::PlayersSkipUpkeep { player } | P::PlayerSkipsDrawStep { player }
        | P::PlayersSkipUntapStep { player }
        | P::PlayersSkipExtraTurns { player } | P::ChoosePlayerAsEnters { filter: player, .. }
        | P::ExileToCounteredExileInsteadOfGraveyard { player, .. }
        | P::PlayerCounterPerTurnLimitReplacement { player_filter: player, .. }
        | P::DoubleTokenCreationReplacement { controller: player, .. }
        | P::RedirectDrawReplacement { drawer: player, .. }
        | P::DoubleLifeChangeReplacement { player, .. } | P::EntersUnderChosenControl(player)
        | P::AddLifeGainReplacement { player, .. } | P::NoMaximumHandSizeFor(player)
        | P::MaximumHandSizeFromSourceCounters { player, .. }
        | P::ExtraDieIgnoreLowest { player, .. }
        | P::ExtraCoinIgnoreOne { player } | P::FirstCoinBatchHeadsWin { player }
        | P::SearchLimitedToTopCards { searcher: player, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        P::DuplicateMatchingTriggeredAbilities { source_filter, event_matcher, .. }
        | P::SuppressMatchingTriggeredAbilities { source_filter, event_matcher, .. } => {
            optional_filter(source_filter, change)?;
            if let Some(trigger) = event_matcher { *trigger = trigger.with_text_change(change)?; }
        }
        P::ExertAttack { linked_trigger, .. } => {
            if let Some(trigger) = linked_trigger { *trigger = rewrite_triggered_ability_words(trigger, change)?; }
        }
        P::EnlistAttack { linked_trigger, .. } => {
            *linked_trigger = rewrite_triggered_ability_words(linked_trigger, change)?;
        }
        P::SetBasePowerToughnessValue { filter, power, toughness } => {
            *filter = rewrite_filter_words(filter, change)?;
            *power = rewrite_value_words(power, change)?;
            *toughness = rewrite_value_words(toughness, change)?;
        }
        P::SourceCharacteristicsOfLastExiledCreatureCard { filter, retained_subtypes: subtypes }
        | P::AddSubtypes { filter, subtypes } | P::SetLandSubtypes { filter, subtypes }
        | P::SetCreatureSubtypes { filter, subtypes }
        | P::EntersWithCharacteristicsForFilter { filter, subtypes, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            subtype_words(subtypes, change);
        }
        P::RemoveCardTypes { filter, condition, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            optional_condition(condition, change)?;
        }
        P::ActivatedAbilityCostReduction {
            filter, reduction, replacement_mana_cost, condition, per_matching_objects,
            per_basic_land_types_among, multiplier, ..
        } => {
            // Native replacement pricing drops a simultaneous generic reduction.
            if replacement_mana_cost.is_some() && *reduction != 0 { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            optional_filter(per_matching_objects, change)?;
            optional_filter(per_basic_land_types_among, change)?;
            optional_value(multiplier, change)?;
            if let Some(condition) = condition { activation_cost_condition_words(condition, change)?; }
        }
        P::ActivatedAbilityCostIncrease { filter, increase, activator, condition, ability_condition, .. } => {
            // The activator constructor replaces the payload's source filter.
            if activator.is_some() && *filter != ObjectFilter::default() { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            *increase = rewrite_total_cost_words(increase, change)?;
            optional_player(activator, change)?;
            optional_condition(condition, change)?;
            if let Some(condition) = ability_condition { activation_cost_condition_words(condition, change)?; }
        }
        P::ChooseColorAsEnters { excluded, .. } => {
            if let Some(color) = excluded { change.replace_color_word(color); }
        }
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { options, .. } => {
            for option in options { statics_words(&mut option.abilities, change)?; }
        }
        P::EnterAsCopyAsEnters { spec, .. } => {
            if spec.added_colors.count() == 5 { return Err(hold(model)); }
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            optional_filter(&mut spec.affected_filter, change)?;
            change.replace_color_words(&mut spec.added_colors);
            subtype_words(&mut spec.added_subtypes, change);
            abilities_words(&mut spec.added_abilities, change)?;
            optional_filter(&mut spec.additional_counters_source_filter, change)?;
            optional_filter(&mut spec.added_abilities_source_filter, change)?;
            for counters in &mut spec.conditional_additional_counters {
                counters.source_filter = rewrite_filter_words(&counters.source_filter, change)?;
            }
            // Durations, copy-source references, follow-up operations, name
            // exceptions, link keys and fixed counter/P/T facts are retained.
        }
        P::AsEntersEffectProgram { program, .. } => *program = rewrite_program_words(program, change)?,
        P::ExileToExileInsteadOfGraveyard { filter, graveyard_owner, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            *graveyard_owner = rewrite_player_filter_words(graveyard_owner, change)?;
        }
        P::ExileWouldDieInstead { filter, damaged_by, damager_filter, exile_with_counters, follow_up_effects, .. } => {
            if damager_filter.is_some() && (damaged_by.is_some() || !exile_with_counters.is_empty()
                || !follow_up_effects.is_empty()) { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            optional_filter(damager_filter, change)?;
            effects_words(follow_up_effects, change)?;
        }
        P::ModifyDamageAmountReplacement { source_filter, target_player_filter, target_object_filter, dynamic_delta, .. } => {
            damage_filters(source_filter, target_player_filter, target_object_filter, change)?;
            optional_value(dynamic_delta, change)?;
        }
        P::MinimumDamageAmountReplacement { source_filter, target_player_filter, target_object_filter, floor, .. } => {
            damage_filters(source_filter, target_player_filter, target_object_filter, change)?;
            *floor = rewrite_value_words(floor, change)?;
        }
        P::DoubleDamageAmountReplacement { source_filter, target_player_filter, target_object_filter, .. }
        | P::PreventHalfDamageReplacement { source_filter, target_player_filter, target_object_filter, .. } => {
            damage_filters(source_filter, target_player_filter, target_object_filter, change)?;
        }
        P::DoubleCountersReplacement { filter, player_filter, actor, includes_permanents, halve, effect_only, .. } => {
            // The legacy constructor choices do not preserve general mixtures
            // of actor scope, player/object scope, or halving. Hold those cases.
            if actor.is_some() || *includes_permanents || *halve
                || (player_filter.is_some() && (*filter != ObjectFilter::default() || *effect_only)) {
                return Err(hold(model));
            }
            *filter = rewrite_filter_words(filter, change)?;
            optional_player(player_filter, change)?;
        }
        P::AddCountersPlacementReplacement { filter, player_filter, .. } => {
            if player_filter.is_some() && *filter != ObjectFilter::default() { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            optional_player(player_filter, change)?;
        }
        P::ActorCountersAddition { filter, player_filter, actor, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
            optional_player(player_filter, change)?;
            *actor = rewrite_player_filter_words(actor, change)?;
        }
        P::MultiplyTokenCreationReplacement { controller, token_filter, .. } => {
            *controller = rewrite_player_filter_words(controller, change)?;
            optional_filter(token_filter, change)?;
        }
        P::AddTokenCreationReplacement { controller, token_filter, additional_token, additional, per_created, .. } => {
            // The Squirrel shortcut also implies green and a type-derived
            // token name without retaining which words were actually authored.
            if *additional_token == ironsmith_core::AdditionalTokenKind::Squirrel
                || (*per_created && *additional != 1) { return Err(hold(model)); }
            *controller = rewrite_player_filter_words(controller, change)?;
            *token_filter = rewrite_filter_words(token_filter, change)?;
        }
        P::CreateOneOfEachTokenReplacement { kinds, .. } => {
            if kinds.contains(&ironsmith_core::AdditionalTokenKind::Squirrel) { return Err(hold(model)); }
        }
        P::KeywordActionReplacement { source_filter, performer_filter, replacement_effects, .. } => {
            *source_filter = rewrite_filter_words(source_filter, change)?;
            optional_player(performer_filter, change)?;
            effects_words(replacement_effects, change)?;
        }
        P::ConditionalDrawReplacement { condition, replacement_effects, .. } => {
            *condition = rewrite_condition_words(condition, change)?;
            effects_words(replacement_effects, change)?;
        }
        P::DrawReplacementWithEffects { drawer, replacement_effects, .. } => {
            *drawer = rewrite_player_filter_words(drawer, change)?;
            effects_words(replacement_effects, change)?;
        }
        P::LoseGameReplacement { replacement_effects, .. } => effects_words(replacement_effects, change)?,
        P::CharacteristicDefiningPt { power, toughness } => {
            *power = rewrite_value_words(power, change)?;
            *toughness = rewrite_value_words(toughness, change)?;
        }
        P::ManaProductionReplacement { source_filter, .. }
        | P::ManaProductionMultiplierReplacement { source_filter, .. }
        | P::PreventDamageToYouFromSourceFilter { source_filter, .. } => {
            *source_filter = rewrite_filter_words(source_filter, change)?;
        }
        P::ManaSpendPermission { permission, .. } => {
            // This symbol selector also encodes prose such as "black mana";
            // no field distinguishes that word from an authored mana symbol.
            if permission.any_color_mana_symbol.is_some() { return Err(hold(model)); }
            permission.player = rewrite_player_filter_words(&permission.player, change)?;
            optional_filter(&mut permission.mana_source_filter, change)?;
            match &mut permission.scope {
                ironsmith_core::ManaSpendScope::ActivationCostsOf(filter)
                | ironsmith_core::ManaSpendScope::CastingSpellsMatching(filter) => {
                    *filter = rewrite_filter_words(filter, change)?;
                }
                ironsmith_core::ManaSpendScope::AllCosts
                | ironsmith_core::ManaSpendScope::CastingSpellsWithStableIds(_) => {}
            }
        }
        P::PreventDamageToSelfRemoveCounter { amount, follow_up, one_damage_per_counter, surface, .. } => {
            if *one_damage_per_counter && (*amount != Value::EventValue(ironsmith_core::EventValueSpec::Amount)
                || follow_up.is_some() || *surface != ironsmith_core::CounterRemovalPreventionSurface::Conjoined) {
                return Err(hold(model));
            }
            *amount = rewrite_value_words(amount, change)?;
        }
        P::PreventConstrainedDamageToSelfPutCountersInstead { source_filter, .. } => optional_filter(source_filter, change)?,
        P::DamagePreventionWithFollowUp { source_filter, target_filter, effects, .. } => {
            *source_filter = rewrite_filter_words(source_filter, change)?;
            *target_filter = rewrite_filter_words(target_filter, change)?;
            effects_words(effects, change)?;
        }
        P::ReplaceDamageWithCountersInstead { source_filter, target_filter, .. } => {
            *source_filter = rewrite_filter_words(source_filter, change)?;
            *target_filter = rewrite_filter_words(target_filter, change)?;
        }
        P::Grants(spec) => {
            spec.filter = rewrite_filter_words(&spec.filter, change)?;
            spec.beneficiary = rewrite_player_filter_words(&spec.beneficiary, change)?;
            optional_filter(&mut spec.cast_this_way_filter, change)?;
            effects_words(&mut spec.on_use_effects, change)?;
            statics_words(&mut spec.cast_this_way_grants, change)?;
            statics_words(&mut spec.permanent_this_way_grants, change)?;
            grantable_words(&mut spec.grantable, change)?;
        }
        P::EntersTappedUnlessCondition { condition, .. } => *condition = rewrite_condition_words(condition, change)?,
        P::EntersWithCountersIfCondition { count, condition, added_abilities, .. } => {
            *count = rewrite_value_words(count, change)?;
            *condition = rewrite_condition_words(condition, change)?;
            abilities_words(added_abilities, change)?;
        }
        P::EntersWithCountersValue { count, .. } | P::EntersWithCounterChoice { count, .. } => {
            *count = rewrite_value_words(count, change)?;
        }
        P::EntersWithCountersAndSubtypesForFilter { filter, count, count_condition, otherwise_count, subtypes, .. } => {
            if count_condition.is_some() != otherwise_count.is_some() { return Err(hold(model)); }
            *filter = rewrite_filter_words(filter, change)?;
            *count = rewrite_value_words(count, change)?;
            optional_condition(count_condition, change)?;
            optional_value(otherwise_count, change)?;
            subtype_words(subtypes, change);
        }
        P::PreventMatchingDamage(spec) => {
            damage_filters(&mut spec.source_filter, &mut spec.target_player_filter, &mut spec.target_object_filter, change)?;
            if let ironsmith_core::StaticDamagePreventionAmount::Amount(value) = &mut spec.amount {
                *value = rewrite_value_words(value, change)?;
            }
        }
        P::PreventMatchingDamageWithFollowUp(spec) => {
            damage_filters(&mut spec.source_filter, &mut spec.target_player_filter, &mut spec.target_object_filter, change)?;
            effects_words(&mut spec.effects, change)?;
        }
        P::RedirectMatchingDamage(spec) => {
            damage_filters(&mut spec.source_filter, &mut spec.target_player_filter, &mut spec.target_object_filter, change)?;
        }
        P::TokenCreationTemplates { controller, token_filter, templates, .. } => {
            *controller = rewrite_player_filter_words(controller, change)?;
            *token_filter = rewrite_filter_words(token_filter, change)?;
            effects_words(templates, change)?;
        }
        P::SpellManaSpendingRestriction(restriction) => match restriction {
            ironsmith_core::mana::ManaSpendingRestriction::ProducedBy(filter) => mana_producer_words(filter, change),
            // The color set conflates an authored list with the word "colored".
            ironsmith_core::mana::ManaSpendingRestriction::OnX { .. } => return Err(hold(model)),
        },
        P::ManaProductionRewrite { rule, .. } => {
            // Symbol input lacks symbol-versus-word provenance. Basic-land
            // arrays cannot represent replacement collisions faithfully.
            if matches!(rule.input, ironsmith_core::mana::ManaRewriteInput::Symbol(_))
                || matches!(rule.output, ironsmith_core::mana::ManaRewriteOutput::ByBasicLandType(_)) {
                return Err(hold(model));
            }
            rule.source_filter = rewrite_filter_words(&rule.source_filter, change)?;
            optional_player(&mut rule.controller, change)?;
        }
        // Complete payloads with no authored words in the supported families.
        // Strings in these variants are presentation or names only; keyword
        // rules, chosen types and actual mana symbols remain untouched.
        P::CountersRemainAcrossZoneChanges { .. }
        | P::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn { .. }
        | P::AttachedChosenLandwalkGrant(_) | P::Dredge(_) | P::CounterLimit { .. }
        | P::CanBlockAdditionalCreatureEachCombat(_) | P::CanBlockAsThoughNoShadow
        | P::CanAttackPlayersWhoAttackedControllerLastTurnAsThoughNoDefender
        | P::CantBeBlockedByMoreThan(_) | P::CantBeBlockedExceptByNOrMore(_)
        | P::CantBeBlockedByPowerOrLess(_) | P::CantBeBlockedByPowerOrGreater(_)
        | P::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(_)
        | P::MayChooseNotToUntapDuringUntapStep(_) | P::PreventAllDamageToYou
        | P::ControlAttachedPermanent(_) | P::CountAsCardNamedForSpellEffect { .. }
        | P::MaxCreaturesCanAttackEachCombat(_) | P::MaxCreaturesCanAttackYouEachCombat(_)
        | P::MaxCreaturesCanAttackSourceEachCombat(_) | P::CanBlockAsThoughUntapped
        | P::CanBlockAsThoughNoLandwalk
        | P::MaxCreaturesCanBlockEachCombat(_) | P::ChooseBasicLandTypeAsEnters(_)
        | P::ChooseLandTypeAsEnters(_) | P::EnchantedLandIsChosenType(_)
        | P::SourceLandIsChosenType(_) | P::SoulbondSharedPowerToughness { .. }
        | P::CostIncreasePerTargetBeyondFirst(_) | P::CostIncreaseManaCostPerTargetBeyondFirst(_)
        | P::AdditionalLifeCostPerTarget(_) | P::CommanderTaxLifeSubstitution { .. }
        | P::MinimumSpellTotalMana(_) | P::BuybackCostReduction(_)
        | P::NoteLifeTotalAsEnters(_) | P::DiscardHandAsEnters(_)
        | P::ChooseCardNameAsEnters { .. } | P::ChooseCreatureTypeAsEnters(_)
        | P::DoubleDamageFromSourcesYouControlOfChosenType(_) | P::AdditionalLandPlays(_)
        | P::RevealFirstCardYouDrawEachTurn { .. } | P::ConditionalSpellKeyword(_)
        | P::DrawExtraCardsReplacement { .. } | P::PayLifeOrEnterTapped(_)
        | P::Bloodthirst(_) | P::Tribute(_) | P::PreventDamageToSelfPutCountersInstead { .. }
        | P::CantAttackYouUnlessControllerPaysPerAttacker(_)
        | P::CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker(_)
        | P::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
        | P::NativeAlternativeCastFromZone { .. } | P::IntrinsicStartingCounters(_)
        | P::ForetellSpecialActionModifier { .. } => {}
        // These variants encode literal qualities or executable prices in an
        // enum name/string without enough authored-word role information.
        P::OpponentsMustTargetFlagbearers | P::FirstEquipCostAlternative(_)
        | P::ChooseNamedOptionAsEnters { .. } | P::ConvertUnspentMana { .. }
        // A generic instead-replacement's event selectors are not yet
        // rewritten; hold rather than change only part of its words.
        | P::EventReplacementWithEffects { .. }
        | P::EchoCostAlternative { .. }
        | P::EventAmountReplacement { .. } => return Err(hold(model)),
    }
    Ok(rewritten)
}

fn damage_filters(source: &mut ObjectFilter, player: &mut Option<PlayerFilter>, object: &mut Option<ObjectFilter>, change: TextChange)
    -> Result<(), Error>
{
    *source = rewrite_filter_words(source, change)?;
    optional_player(player, change)?;
    optional_filter(object, change)
}

fn wordless_leaf(id: Option<StaticAbilityId>) -> bool {
    matches!(id, Some(
        StaticAbilityId::Flying | StaticAbilityId::FirstStrike | StaticAbilityId::DoubleStrike
        | StaticAbilityId::Deathtouch | StaticAbilityId::Defender | StaticAbilityId::Flash
        | StaticAbilityId::Haste | StaticAbilityId::Hexproof | StaticAbilityId::Indestructible
        | StaticAbilityId::Intimidate | StaticAbilityId::Lifelink | StaticAbilityId::Menace
        | StaticAbilityId::Banding | StaticAbilityId::Reach | StaticAbilityId::Shroud
        | StaticAbilityId::Trample | StaticAbilityId::Vigilance | StaticAbilityId::Fear
        | StaticAbilityId::Skulk | StaticAbilityId::Wither | StaticAbilityId::Infect
        | StaticAbilityId::Changeling | StaticAbilityId::Phasing
        | StaticAbilityId::Shadow | StaticAbilityId::Horsemanship | StaticAbilityId::Prowess
        | StaticAbilityId::Flanking | StaticAbilityId::UmbraArmor | StaticAbilityId::LivingMetal
        | StaticAbilityId::Daybound | StaticAbilityId::Nightbound | StaticAbilityId::Ascend
        | StaticAbilityId::Storied | StaticAbilityId::SplitSecond | StaticAbilityId::Rebound
        | StaticAbilityId::Cascade | StaticAbilityId::ReadAhead | StaticAbilityId::Unleash
        | StaticAbilityId::Delve | StaticAbilityId::Convoke | StaticAbilityId::Improvise
    ))
}

fn activation_cost_condition_words(condition: &mut ironsmith_core::ActivatedAbilityCostCondition, change: TextChange)
    -> Result<(), Error>
{
    use ironsmith_core::ActivatedAbilityCostCondition as C;
    match condition {
        C::TargetsExactly { filter, .. } => *filter = rewrite_filter_words(filter, change)?,
        C::EquipAbility { targeting } => optional_filter(targeting, change)?,
        C::ThisAbility { .. } | C::Keyword(_) | C::NonManaAbility | C::LoyaltyAbility
        | C::FirstKeywordAbilityThisTurn { .. } => {}
        C::Activator(player) => *player = rewrite_player_filter_words(player, change)?,
        C::All(conditions) => {
            for condition in conditions { activation_cost_condition_words(condition, change)?; }
        }
    }
    Ok(())
}

fn spell_cost_condition_words(condition: &mut ThisSpellCostCondition, change: TextChange) -> Result<(), Error> {
    use ThisSpellCostCondition as C;
    match condition {
        C::ConditionExpr { condition, .. } | C::AsLongAsConditionExpr { condition, .. } => {
            *condition = rewrite_condition_words(condition, change)?;
        }
        C::TargetsPlayer(player) => *player = rewrite_player_filter_words(player, change)?,
        C::TargetsObject(filter) | C::TargetsObjectWhoseControllerHasCardsInGraveyardOrMore { filter, .. }
        | C::NoCardsInHandMatching { filter, .. } | C::CardInYourGraveyardMatching { filter, .. } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        C::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(subtype)
        | C::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(subtype) => {
            change.replace_subtype_word(subtype);
        }
        C::Always | C::YourTurn | C::NotYourTurn | C::YouLifeTotalOrLess(_)
        | C::OpponentHasNoCardsInHand | C::OpponentControlsLandsOrMore(_)
        | C::OpponentControlsAtLeastNMoreCreaturesThanYou(_)
        | C::TotalCreatureCardsInAllGraveyardsOrMore(_) | C::OpponentCastSpellsThisTurnOrMore(_)
        | C::OpponentDrewCardsThisTurnOrMore(_) | C::OpponentHadCardsPutIntoGraveyardThisTurnOrMore(_)
        | C::YouWereDealtDamageByCreaturesThisTurnOrMore(_) | C::FirstSpellYouCastThisGame
        | C::YouCastSpellsThisTurnOrMore { .. } | C::YouGainedLifeThisTurnOrMore(_)
        | C::OpponentHasPoisonCountersOrMore(_) | C::OpponentHasCardsInGraveyardOrMore(_)
        | C::DistinctCardTypesInYourGraveyardOrMore(_) | C::LifeTotalLessThanStarting | C::IsNight
        | C::YouSacrificedArtifactThisTurn | C::YouCommittedCrimeThisTurn
        | C::CreatureLeftBattlefieldUnderYourControlThisTurn | C::YouHaveCardsInYourGraveyardOrMore(_)
        | C::YouHaveCardsOfTypesInYourGraveyardOrMore { .. } | C::OnlyCreatureCardsInHandNamed(_)
        | C::NotStartingPlayer | C::CreatureCardPutIntoYourGraveyardThisTurn | C::CreatureIsAttackingYou
        | C::YouDealtCombatDamageToPlayerSharingCreatureTypeThisTurn => {}
    }
    Ok(())
}

fn attack_condition_words(condition: &mut ironsmith_core::CantAttackUnlessConditionSpec, change: TextChange)
    -> Result<(), Error>
{
    use ironsmith_core::{AttackCostCondition as Cost, AttackingGroupAttackCondition as Group,
        CantAttackUnlessConditionSpec as C, DefendingPlayerAttackCondition as Defender};
    match condition {
        C::AttackCost(cost) => match cost {
            Cost::ReturnPermanentsToOwnersHand { filter, .. } | Cost::SacrificePermanents { filter, .. } => {
                *filter = rewrite_filter_words(filter, change)?;
            }
            Cost::PayGenericPerSourceCounter { .. } => {}
        },
        C::AttackingGroupCondition(group) => match group {
            // The model cannot express a replacement of either literal color.
            Group::BlackOrGreenCreatureAlsoAttacks => return Err(Error::Condition),
            Group::AtLeastNOtherCreaturesAttack(_) | Group::CreatureWithGreaterPowerAlsoAttacks
            | Group::AtLeastNOtherCreaturesBlock(_) | Group::CreatureWithGreaterPowerAlsoBlocks => {}
        },
        C::BattlefieldCountAtLeast { filter, .. } | C::ControllerControlsMoreThanDefendingPlayer(filter) => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        C::DefendingPlayerCondition(condition) => match condition {
            Defender::Controls(filter) => *filter = rewrite_filter_words(filter, change)?,
            Defender::ControlsEnchantmentOrEnchantedPermanent | Defender::HasCardsInGraveyardOrMore(_)
            | Defender::IsMonarch | Defender::IsPoisoned => {}
        },
        C::SourceCondition(condition) => *condition = rewrite_condition_words(condition, change)?,
        C::ControllerGraveyardHasCardsAtLeast(_) | C::OpponentWasDealtDamageThisTurn => {}
    }
    Ok(())
}

fn restriction_words(restriction: &mut ironsmith_core::Restriction, change: TextChange) -> Result<(), Error> {
    use ironsmith_core::Restriction as R;
    match restriction {
        R::AdditionalLandPlays(player, _) | R::NoMaximumHandSize(player) | R::GainLife(player)
        | R::DrawFromBottom(player) | R::ActivateAbilities(player)
        | R::SearchLibraries(player) | R::SearchOwnLibraryFromOwnEffects(player)
        | R::CastSpellsOnlyAsSorcery(player) | R::ActivateNonManaAbilities(player)
        | R::DrawCards(player) | R::DrawExtraCards(player) | R::PoisonCounters(player)
        | R::LoseLife(player) | R::DamageCauseLifeLoss(player) | R::DamageReduceLifeBelowOne(player)
        | R::ChangeLifeTotal(player) | R::LoseGame(player) | R::LoseGameForZeroLife(player)
        | R::WinGame(player) | R::BecomeMonarch(player) | R::BeTargetedPlayer(player)
        | R::VentureMoreThanOnceEachTurn(player) | R::BlockWithMoreThan { player, .. } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        R::CastSpellsMatching(player, filter) | R::CastMoreThanOneSpellEachTurn(player, filter)
        | R::CastMoreThanNSpellsEachTurn { player, spells: filter, .. }
        | R::BeTargetedPlayerFrom(player, filter) | R::PlayerHexproofFrom(player, filter)
        | R::PlayLandsMatching(player, filter)
        | R::AttackPlayerOrPlaneswalkersControlledBy { attackers: filter, player }
        | R::AttackPlayer { attackers: filter, player }
        | R::MustAttackPlayer { attackers: filter, player } => {
            *player = rewrite_player_filter_words(player, change)?;
            *filter = rewrite_filter_words(filter, change)?;
        }
        R::LoseUnspentMana(player, color) => {
            *player = rewrite_player_filter_words(player, change)?;
            if let Some(color) = color { change.replace_color_word(color); }
        }
        R::ActivateAbilitiesOf(filter) | R::ActivateTapAbilitiesOf(filter)
        | R::ActivateNonManaAbilitiesOf(filter) | R::Attack(filter) | R::AttackAlone(filter)
        | R::Block(filter) | R::MustBeBlocked(filter) | R::BlockAlone(filter) | R::Untap(filter)
        | R::BeBlocked(filter) | R::BeDestroyed(filter) | R::BeRegenerated(filter)
        | R::BeSacrificed(filter) | R::HaveCountersPlaced(filter) | R::HaveCounterTypePlaced(filter, _)
        | R::BeTargeted(filter) | R::BeCountered(filter) | R::Transform(filter) | R::TurnFaceUp(filter)
        | R::PhaseOut(filter) | R::PhaseIn(filter) | R::AttackOrBlock(filter)
        | R::AttackOrBlockAlone(filter) | R::EnterBattlefield(filter)
        | R::PreventDamageFrom { sources: filter, .. } | R::ActivateLoyaltyAbilitiesOf(filter)
        | R::MustAttack(filter) | R::BecomeSuspected(filter) | R::BecomeUntapped(filter)
        | R::AttackBlockOrCrew(filter)
        | R::MaximumBlockers { filter, .. }
        | R::MustBlock(filter) => *filter = rewrite_filter_words(filter, change)?,
        R::BlockSpecificAttacker { blockers, attacker } | R::MustBlockSpecificAttacker { blockers, attacker }
        | R::BeTargetedFrom(blockers, attacker) | R::BeAttachedBy(blockers, attacker)
        | R::AttackPermanents { attackers: blockers, permanents: attacker } => {
            *blockers = rewrite_filter_words(blockers, change)?;
            *attacker = rewrite_filter_words(attacker, change)?;
        }
        R::BeSacrificedByCause { filter, cause } => {
            *filter = rewrite_filter_words(filter, change)?;
            optional_filter(&mut cause.source_filter, change)?;
            // Cause controller is a closed relation, not a PlayerFilter.
        }
        R::AttackTax(rule) => rule.attackers = rewrite_filter_words(&rule.attackers, change)?,
        R::PreventDamage | R::PreventCombatDamage | R::AttackYouUnlessControllerPaysPerAttacker(_, _) => {}
    }
    Ok(())
}

fn mana_producer_words(filter: &mut ironsmith_core::mana::ManaProducerFilter, change: TextChange) {
    use ironsmith_core::mana::ManaProducerFilter as F;
    match filter {
        F::Subtype(subtype) => change.replace_subtype_word(subtype),
        F::All(filters) => { for filter in filters { mana_producer_words(filter, change); } }
        F::CardType(_) | F::Supertype(_) => {}
    }
}

fn grantable_words(
    grantable: &mut ironsmith_core::Grantable<CompiledStaticAbility, Effect, Cost, ThisSpellCostCondition>,
    change: TextChange,
) -> Result<(), Error> {
    use ironsmith_core::{AlternativeCastingMethod as A, DerivedAlternativeCast as D, Grantable as G};
    match grantable {
        G::Ability(model) => *model = rewrite_static_model_words(model, change)?,
        G::PlayFrom => {}
        G::AlternativePrice { costs, .. } => {
            *costs = costs.iter().map(|cost| rewrite_cost_words(cost, change)).collect::<Result<_, _>>()?;
        }
        G::DerivedAlternativeCast(derived) => match derived {
            D::FlashbackFromCardManaCost { additional_costs } => {
                *additional_costs = additional_costs.iter().map(|cost| rewrite_cost_words(cost, change)).collect::<Result<_, _>>()?;
            }
            D::GraveyardCastFromCardManaCost { additional_costs, condition, .. } => {
                *additional_costs = additional_costs.iter().map(|cost| rewrite_cost_words(cost, change)).collect::<Result<_, _>>()?;
                if let Some(condition) = condition { spell_cost_condition_words(condition, change)?; }
            }
            // These derive cost/ability rules from a named keyword. Those
            // implied rules do not acquire authored type/color occurrences.
            D::RetraceFromCardManaCost | D::BlitzFromCardManaCost | D::EmergeFromCardManaCost
            | D::MiracleFromCardManaCostReducedBy { .. } | D::EscapeFromCardManaCost { .. }
            | D::ManaValueAsGenericFromHand | D::LifeEqualManaValueFromHand { .. }
            | D::LifeEqualManaValueFromZone { .. } | D::MadnessFromCardManaCost => {}
        },
        G::AlternativeCast(method) => match method {
            A::Blitz { total_cost } | A::Flashback { total_cost, .. } | A::Harmonize { total_cost }
            | A::Retrace { total_cost } | A::Madness { total_cost } | A::Bestow { total_cost }
            | A::FlashWithAdditionalCost { total_cost, .. } => {
                *total_cost = rewrite_total_cost_words(total_cost, change)?;
            }
            A::Warp { additional_cost, .. } | A::JumpStart { additional_cost }
            | A::Escape { additional_cost, .. } => {
                *additional_cost = rewrite_total_cost_words(additional_cost, change)?;
            }
            A::Overload { effects, .. } | A::Cleave { effects, .. } => effects_words(effects, change)?,
            A::FromZone { total_cost, condition, .. } => {
                *total_cost = rewrite_total_cost_words(total_cost, change)?;
                if let Some(condition) = condition { spell_cost_condition_words(condition, change)?; }
            }
            // Composed still dispatches keyword semantics by name. Awaken's
            // program contains a generated Elemental instruction; there is no
            // distinction between that implied type and an authored one.
            A::Composed { .. } | A::Awaken { .. } => return Err(Error::Cost),
            A::Dash { .. } | A::Plot { .. } | A::Suspend { .. } | A::Disturb { .. }
            | A::Miracle { .. } | A::Foretell { .. } | A::Trap { .. } | A::Mutate { .. } => {}
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    // Source-only regression witnesses. These were authored without running
    // a compiler, tests, formatter, engine, or corpus under the campaign hold.
    use super::*;
    use crate::cost::TotalCost;
    use crate::effects::DrawCardsEffect;
    use crate::static_abilities::StaticAbility;
    use ironsmith_core::{Color, ColorSet, ManaCost, ManaSymbol, Zone};

    fn change() -> TextChange { TextChange::color(Color::Black, Color::Blue).unwrap() }
    fn black() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLACK), ..ObjectFilter::default() } }
    fn blue() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() } }
    fn word_effect() -> Effect { Effect::new(DrawCardsEffect::you(Value::Count(black()))) }
    fn pair() -> ironsmith_core::LinkedExilePair {
        ironsmith_core::LinkedExilePair { definition: ironsmith_core::LinkedExileDefinition([9; 32]), pair: 71 }
    }
    fn model(payload: P<Trigger, Effect, Cost, ThisSpellCostCondition>) -> CompiledStaticAbility {
        CompiledStaticAbility { id: None, label: "Black Knight's unchanged label".into(), payload }
    }

    #[test]
    fn nested_attachment_definition_keeps_links_presentation_mana_and_earlier_capture() {
        let program = crate::resolution::ResolutionProgram::from_effects(vec![word_effect()])
            .with_linked_exile_pair(pair());
        let mut ability = AbilityModel::activated(TotalCost::from_cost(Cost::sacrifice(black())), program)
            .in_zones(vec![Zone::Graveyard]);
        let ironsmith_core::AbilityKind::Activated(activated) = &mut ability.kind else { unreachable!() };
        activated.choices.push(ironsmith_core::ChooseSpec::target(ironsmith_core::ChooseSpec::Object(black())));
        activated.activation_condition = Some(Condition::YouControl(black()));
        activated.mana_output = Some(vec![ManaSymbol::Black]);
        let original = model(P::Conditional {
            condition: Condition::YouControl(black()),
            ability: Box::new(model(P::AttachedAbilityGrant(Box::new(
                ironsmith_core::AttachedAbilityGrant::new(ability, "Black Knight has this ability")
                    .with_condition(Condition::YouControl(black()))
                    .with_protection_attachment_exception(true),
            )))),
        });
        let captured = original.clone();
        let changed = rewrite_static_model_words(&original, change()).unwrap();
        assert_eq!(changed.label, "Black Knight's unchanged label");
        assert_eq!(changed.id, original.id);
        let P::Conditional { ability, condition } = &changed.payload else { unreachable!() };
        assert_eq!(condition, &Condition::YouControl(blue()));
        let P::AttachedAbilityGrant(grant) = &ability.payload else { unreachable!() };
        assert_eq!(grant.display, "Black Knight has this ability");
        assert!(grant.protection_does_not_remove_controlled_attachments);
        assert_eq!(grant.condition, Some(Condition::YouControl(blue())));
        assert_eq!(grant.ability.functional_zones, vec![Zone::Graveyard]);
        let ironsmith_core::AbilityKind::Activated(activated) = &grant.ability.kind else { unreachable!() };
        assert_eq!(activated.effects.linked_exile_pair, Some(pair()));
        assert_eq!(activated.mana_output, Some(vec![ManaSymbol::Black]));
        assert_eq!(activated.activation_condition, Some(Condition::YouControl(blue())));
        assert_eq!(activated.choices, vec![ironsmith_core::ChooseSpec::target(ironsmith_core::ChooseSpec::Object(blue()))]);
        assert_eq!(activated.mana_cost.costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue())));
        assert_eq!(activated.effects.flattened_default_effects()[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        let P::Conditional { ability, condition } = &captured.payload else { unreachable!() };
        assert_eq!(condition, &Condition::YouControl(black()));
        let P::AttachedAbilityGrant(grant) = &ability.payload else { unreachable!() };
        let ironsmith_core::AbilityKind::Activated(activated) = &grant.ability.kind else { unreachable!() };
        assert_eq!(activated.effects.flattened_default_effects()[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(black()));
    }

    #[test]
    fn changed_static_model_materializes_changed_native_layer_effects() {
        let original = model(P::SetColors { filter: black(), colors: ColorSet::BLACK, exclude_from_color_identity: false });
        let changed = rewrite_static_model_words(&original, change()).unwrap();
        let source = crate::ids::ObjectId::from_raw(301);
        let controller = crate::ids::PlayerId::from_index(0);
        let game = crate::game_state::GameState::new(vec!["A".into(), "B".into()], 20);
        let effects = StaticAbility::from_model(changed).generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0].applies_to, crate::continuous::EffectTarget::Filter(filter) if filter == &blue()));
        assert!(matches!(&effects[0].modification, crate::continuous::Modification::SetColors(colors) if *colors == ColorSet::BLUE));
        let earlier = StaticAbility::from_model(original).generate_effects(source, controller, &game);
        assert!(matches!(&earlier[0].modification, crate::continuous::Modification::SetColors(colors) if *colors == ColorSet::BLACK));
    }

    #[test]
    fn linked_cast_permission_rewrites_whole_price_scope_and_on_use_definition() {
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Black]]);
        let mut permission = ironsmith_core::GrantSpec::new(ironsmith_core::Grantable::AlternativePrice {
            costs: vec![Cost::mana(mana.clone()), Cost::sacrifice(black())], origin: Some(Zone::Exile),
        }, black(), Zone::Exile);
        permission.beneficiary = PlayerFilter::ControlsMost { filter: Box::new(black()) };
        permission.requires_linked_exile_pair = true;
        permission.may_look_at_linked_exile = true;
        permission.linked_exile_pair = Some(pair());
        permission.max_plays = Some(2);
        permission.filtered_zone_surface = Some("black is presentation".into());
        permission.cast_this_way_filter = Some(black());
        permission.cast_this_way_grants.push(CompiledStaticAbility::protection(ironsmith_core::ProtectionFrom::Color(ColorSet::BLACK)));
        permission.on_use_effects.push(word_effect());
        let changed = rewrite_static_model_words(&model(P::Grants(Box::new(permission))), change()).unwrap();
        let P::Grants(permission) = changed.payload else { unreachable!() };
        assert_eq!(permission.filter, blue());
        assert_eq!(permission.cast_this_way_filter, Some(blue()));
        assert_eq!(permission.beneficiary, PlayerFilter::ControlsMost { filter: Box::new(blue()) });
        assert_eq!(permission.linked_exile_pair, Some(pair()));
        assert!(permission.requires_linked_exile_pair && permission.may_look_at_linked_exile);
        assert_eq!(permission.max_plays, Some(2));
        assert_eq!(permission.filtered_zone_surface.as_deref(), Some("black is presentation"));
        let ironsmith_core::Grantable::AlternativePrice { costs, origin } = permission.grantable else { unreachable!() };
        assert_eq!(origin, Some(Zone::Exile));
        assert_eq!(costs[0].compiled_model(), Some(&ironsmith_core::Cost::Mana(mana)));
        assert_eq!(costs[1].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue())));
        assert_eq!(permission.on_use_effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert!(matches!(&permission.cast_this_way_grants[0].payload, P::Protection(ironsmith_core::ProtectionFrom::Color(colors)) if *colors == ColorSet::BLUE));
    }

    #[test]
    fn prevention_follow_up_keeps_amount_basis_and_captured_tag() {
        let changed = rewrite_static_model_words(&model(P::PreventMatchingDamageWithFollowUp(
            ironsmith_core::StaticDamagePreventionFollowUp {
                source_filter: black(), target_player_filter: Some(PlayerFilter::ChosenPlayer),
                target_object_filter: Some(black()), combat_only: true, noncombat_only: false,
                damage_source_tag: Some("black-damage-source".into()), effects: vec![word_effect()],
                display: "black-damage-source is an identity".into(),
                amount_basis: ironsmith_core::PreventionFollowUpAmount::Proposed,
            },
        )), change()).unwrap();
        let P::PreventMatchingDamageWithFollowUp(spec) = changed.payload else { unreachable!() };
        assert_eq!(spec.source_filter, blue());
        assert_eq!(spec.target_object_filter, Some(blue()));
        assert_eq!(spec.target_player_filter, Some(PlayerFilter::ChosenPlayer));
        assert_eq!(spec.damage_source_tag.unwrap().as_str(), "black-damage-source");
        assert_eq!(spec.amount_basis, ironsmith_core::PreventionFollowUpAmount::Proposed);
        assert!(spec.combat_only && !spec.noncombat_only);
        assert_eq!(spec.effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
    }

    #[test]
    fn entry_program_retains_linked_pair_and_source_surface_through_self_replacement() {
        let program = crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![word_effect()], starts_new_source_line: true,
            self_replacements: vec![crate::resolution::SelfReplacementBranch {
                condition: Condition::YouControl(black()), replacement_effects: vec![word_effect()],
                presentation_label: None, condition_after_replacement: true,
                leading_instead_surface: true, starts_new_source_line: true,
            }],
        }]).with_linked_exile_pair(pair());
        let original = CompiledStaticAbility::as_enters_effect_program(
            program, "Black Knight", true, true, None,
        ).with_entry_instead_surface();
        let changed = rewrite_static_model_words(&original, change()).unwrap();
        let P::AsEntersEffectProgram {
            program, subject, also_turns_face_up, turns_face_up_only,
            uses_enters_with_counter_surface, entry_instead_surface, ..
        } = changed.payload else { unreachable!() };
        assert_eq!(subject, "Black Knight");
        assert!(also_turns_face_up && uses_enters_with_counter_surface && entry_instead_surface);
        assert!(!turns_face_up_only);
        assert_eq!(program.linked_exile_pair, Some(pair()));
        assert!(program.segments[0].starts_new_source_line);
        let branch = &program.segments[0].self_replacements[0];
        assert_eq!(branch.condition, Condition::YouControl(blue()));
        assert!(branch.condition_after_replacement && branch.leading_instead_surface && branch.starts_new_source_line);
        assert_eq!(branch.replacement_effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert_eq!(program.flattened_default_effects()[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
    }

    #[test]
    fn landwalk_and_cost_condition_words_change_but_runtime_selected_types_do_not() {
        let mut filter = ObjectFilter::default();
        filter.chosen_creature_type = true;
        let original = model(P::AddChosenCreatureType { filter, display: "chosen Elf is not an authored literal".into() });
        let changed = rewrite_static_model_words(&original, TextChange::creature_type(Subtype::Elf, Subtype::Vampire).unwrap()).unwrap();
        let P::AddChosenCreatureType { filter, display } = changed.payload else { unreachable!() };
        assert!(filter.chosen_creature_type);
        assert_eq!(display, "chosen Elf is not an authored literal");
        let original = model(P::BlockingAsThoughNoLandwalk(ironsmith_core::static_ability_model::BlockingAsThoughNoLandwalkSpec {
            objects: ObjectFilter::default(),
            landwalk: Some(ironsmith_core::LandwalkKind::Subtype { subtype: Subtype::Island, snow: true }),
            display: "Island is presentation".into(),
        }));
        let changed = rewrite_static_model_words(&original, TextChange::basic_land_type(Subtype::Island, Subtype::Forest).unwrap()).unwrap();
        let P::BlockingAsThoughNoLandwalk(spec) = changed.payload else { unreachable!() };
        assert_eq!(spec.landwalk, Some(ironsmith_core::LandwalkKind::Subtype { subtype: Subtype::Forest, snow: true }));
        let mut condition = ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(Subtype::Elf);
        spell_cost_condition_words(&mut condition, TextChange::creature_type(Subtype::Elf, Subtype::Vampire).unwrap()).unwrap();
        assert_eq!(condition, ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeThisTurn(Subtype::Vampire));
    }

    #[test]
    fn opaque_child_and_lossy_native_combinations_hold_without_mutating_parent() {
        let original = model(P::EquipmentGrant(vec![
            CompiledStaticAbility::protection(ironsmith_core::ProtectionFrom::Color(ColorSet::BLACK)),
            model(P::ChooseNamedOptionAsEnters { options: vec!["black".into()], display: String::new(), at_random: false }),
        ]));
        assert!(matches!(rewrite_static_model_words(&original, change()), Err(Error::StaticAbility(_))));
        let P::EquipmentGrant(children) = &original.payload else { unreachable!() };
        assert!(matches!(&children[0].payload, P::Protection(ironsmith_core::ProtectionFrom::Color(colors)) if *colors == ColorSet::BLACK));
        let all_colors: ColorSet = Color::ALL.into_iter().collect();
        let lossy = [
            model(P::SetColors { filter: black(), colors: all_colors, exclude_from_color_identity: false }),
            model(P::ExileWouldDieInstead {
                filter: black(), damaged_by: None, damager_filter: Some(black()), damager_filter_surface: None,
                exile_with_counters: vec![], follow_up_effects: vec![word_effect()],
            }),
            model(P::EntersWithCountersAndSubtypesForFilter {
                filter: black(), counter: ironsmith_core::CounterType::PlusOnePlusOne,
                count: Value::Fixed(1), count_condition: Some(Condition::YouControl(black())),
                otherwise_count: None, subtypes: vec![Subtype::Elf],
            }),
            model(P::OpponentsMustTargetFlagbearers),
        ];
        for original in lossy {
            assert!(matches!(rewrite_static_model_words(&original, change()), Err(Error::StaticAbility(_))));
        }
    }
    #[test]
    fn immutable_static_rewrites_retain_nested_grant_occurrences_and_old_captures() {
        let original = StaticAbility::from_model(model(P::EquipmentGrant(vec![
            CompiledStaticAbility::protection(ironsmith_core::ProtectionFrom::Color(ColorSet::BLACK)),
        ])));
        let captured = original.clone();
        let first = original.with_text_change(change()).unwrap();
        let repeated = original.with_text_change(change()).unwrap();
        assert!(std::sync::Arc::ptr_eq(&first.0, &repeated.0));
        assert_eq!(first.instance_id(), original.instance_id());
        let first_child = &first.equipment_grant_abilities().unwrap()[0];
        let repeated_child = &repeated.equipment_grant_abilities().unwrap()[0];
        assert_eq!(first_child.instance_id(), repeated_child.instance_id());
        assert_eq!(first_child.protection_from(), Some(&ironsmith_core::ProtectionFrom::Color(ColorSet::BLUE)));
        assert_eq!(captured.equipment_grant_abilities().unwrap()[0].protection_from(),
            Some(&ironsmith_core::ProtectionFrom::Color(ColorSet::BLACK)));
        let second = first.with_text_change(TextChange::color(Color::Blue, Color::Green).unwrap()).unwrap();
        let again = repeated.with_text_change(TextChange::color(Color::Blue, Color::Green).unwrap()).unwrap();
        assert_eq!(second.equipment_grant_abilities().unwrap()[0].instance_id(),
            again.equipment_grant_abilities().unwrap()[0].instance_id());
        assert_eq!(second.equipment_grant_abilities().unwrap()[0].protection_from(),
            Some(&ironsmith_core::ProtectionFrom::Color(ColorSet::GREEN)));
    }

    #[test]
    fn a_replaced_public_static_executor_cannot_reuse_its_old_text_cache() {
        let mut original = StaticAbility::protection(ironsmith_core::ProtectionFrom::Color(ColorSet::BLACK));
        let _first = original.with_text_change(change()).unwrap();
        original.0 = StaticAbility::protection(ironsmith_core::ProtectionFrom::Color(ColorSet::GREEN)).0;
        let current = original.with_text_change(change()).unwrap();
        assert_eq!(current.protection_from(), Some(&ironsmith_core::ProtectionFrom::Color(ColorSet::GREEN)));
    }

    #[test]
    fn static_type_word_sets_collapse_duplicate_destinations() {
        let original = model(P::AddSubtypes { filter: ObjectFilter::creature(),
            subtypes: vec![Subtype::Human, Subtype::Vampire] });
        let changed = rewrite_static_model_words(&original,
            TextChange::creature_type(Subtype::Human, Subtype::Vampire).unwrap()).unwrap();
        let P::AddSubtypes { subtypes, .. } = changed.payload else { unreachable!() };
        assert_eq!(subtypes, vec![Subtype::Vampire]);
    }

}

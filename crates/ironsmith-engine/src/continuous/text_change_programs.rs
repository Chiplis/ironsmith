//! Typed authored-word traversal for immutable executable definitions.
//!
//! Each admitted executor is cloned in full before its typed word-bearing
//! fields are visited. Unknown executors fail closed. Display text, retained
//! JSON, names, binding tags and previously selected values are never a source
//! of executable semantics. All nested effects use the immutable memoizing
//! owner so repeated layer reads retain their executor identities.

use super::text_change_predicates::{
    rewrite_aggregate_constraint_words, rewrite_choose_spec_words,
    rewrite_condition_words, rewrite_filter_words, rewrite_player_filter_words,
    rewrite_value_words,
};
use super::text_changes::TextChangeDomainError as Error;
use crate::ability::{Ability, AbilityKind, ManaUsageRestriction};
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, EffectPredicate};
use crate::effects::*;
use crate::resolution::ResolutionProgram;
use ironsmith_core::{ManaPaymentPredicate, ManaUsageSubtypeRequirement, TextChange};

#[cfg(test)]
#[path = "text_change_token_tests.rs"]
mod token_tests;

pub(crate) fn rewrite_program_words(
    program: &ResolutionProgram,
    change: TextChange,
) -> Result<ResolutionProgram, Error> {
    if !program.has_complete_definition() { return Err(Error::SpellProgram); }
    // The shared mapper retains linked_exile_pair and every segment/branch
    // presentation field, and refreshes the flattened default instructions.
    let mut rewritten = program.clone().try_map_effects(|effect| effect.with_text_change(change))?;
    for segment in &mut rewritten.segments {
        for branch in &mut segment.self_replacements {
            branch.condition = rewrite_condition_words(&branch.condition, change)?;
        }
    }
    Ok(rewritten)
}

pub fn rewrite_ability_words(ability: &Ability, change: TextChange) -> Result<Ability, Error> {
    let mut rewritten = ability.clone();
    match &mut rewritten.kind {
        AbilityKind::Static(ability) => *ability = ability.with_text_change(change)?,
        AbilityKind::Triggered(ability) => *ability = rewrite_triggered_ability_words(ability, change)?,
        AbilityKind::Activated(ability) => {
            // Legacy executable restrictions have no typed word provenance.
            if !ability.additional_restrictions.is_empty() { return Err(Error::ActivatedAbility); }
            ability.mana_cost = rewrite_total_cost_words(&ability.mana_cost, change)?;
            ability.effects = rewrite_program_words(&ability.effects, change)?;
            ability.choices = rewrite_choices(&ability.choices, change)?;
            ability.activation_restrictions = ability.activation_restrictions.iter()
                .map(|condition| rewrite_condition_words(condition, change)).collect::<Result<_, _>>()?;
            ability.activation_condition = ability.activation_condition.as_ref()
                .map(|condition| rewrite_condition_words(condition, change)).transpose()?;
            ability.mana_usage_restrictions = ability.mana_usage_restrictions.iter()
                .map(|restriction| rewrite_mana_restriction(restriction, change)).collect::<Result<_, _>>()?;
            // Mana output contains symbols, not authored color words. Timing,
            // keyword identity and source-zone metadata are retained by clone.
        }
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_triggered_ability_words(ability: &crate::ability::TriggeredAbility, change: TextChange)
    -> Result<crate::ability::TriggeredAbility, Error>
{
    // This helper also visits quoted future grants, which have an authored
    // occurrence but no acquisition yet. Active rules text proves acquisition
    // separately at the Layer-3 entry point.
    if ability.effects.retained_trigger_definition().is_none() {
        return Err(Error::TriggeredAbility);
    }
    let mut rewritten = ability.clone();
    rewritten.trigger = ability.trigger.with_text_change(change)?;
    rewritten.effects = rewrite_program_words(&ability.effects, change)?;
    rewritten.choices = rewrite_choices(&ability.choices, change)?;
    rewritten.intervening_if = ability.intervening_if.as_ref()
        .map(|condition| rewrite_condition_words(condition, change)).transpose()?;
    Ok(rewritten)
}

fn rewrite_effects(effects: &[Effect], change: TextChange) -> Result<Vec<Effect>, Error> {
    effects.iter().map(|effect| effect.with_text_change(change)).collect()
}

fn rewrite_choices(choices: &[crate::target::ChooseSpec], change: TextChange)
    -> Result<Vec<crate::target::ChooseSpec>, Error>
{
    choices.iter().map(|choice| rewrite_choose_spec_words(choice, change)).collect()
}

pub(crate) fn rewrite_total_cost_words(cost: &TotalCost, change: TextChange) -> Result<TotalCost, Error> {
    cost.clone().try_map(|component| rewrite_cost_words(&component, change))
}

fn same_total_cost_identity(left: &TotalCost, right: &TotalCost) -> bool {
    // Runtime Cost::PartialEq compares rendered labels. It cannot establish
    // whether rewriting changed a cost definition.
    use ironsmith_core::TotalCostKind;
    match (left.kind(), right.kind()) {
        (TotalCostKind::All(left), TotalCostKind::All(right)) => left.len() == right.len()
            && left.iter().zip(right).all(|(left, right)| std::sync::Arc::ptr_eq(&left.0, &right.0)),
        (TotalCostKind::OneOf(left), TotalCostKind::OneOf(right)) => left.len() == right.len()
            && left.iter().zip(right).all(|(left, right)| same_total_cost_identity(left, right)),
        _ => false,
    }
}

pub(crate) fn rewrite_cost_words(cost: &Cost, change: TextChange) -> Result<Cost, Error> {
    let Some(model) = cost.compiled_model() else {
        // Unmodeled payers may carry extra payment behavior beyond their
        // exposed effect. A partial view is not a complete cost definition.
        return Err(Error::Cost);
    };
    let mut rewritten = model.clone();
    match &mut rewritten {
        ironsmith_core::Cost::DynamicMana(dynamic) => {
            dynamic.source_mana_cost_reduction_condition = dynamic.source_mana_cost_reduction_condition.as_ref()
                .map(|condition| rewrite_condition_words(condition, change).map(Box::new)).transpose()?;
            for value in [&mut dynamic.x_value, &mut dynamic.additional_generic, &mut dynamic.multiplier] {
                *value = value.as_ref().map(|value| rewrite_value_words(value, change)).transpose()?;
            }
            dynamic.mana_cost_of = dynamic.mana_cost_of.as_ref()
                .map(|spec| rewrite_choose_spec_words(spec, change).map(Box::new)).transpose()?;
        }
        ironsmith_core::Cost::Sacrifice(filter) => *filter = rewrite_filter_words(filter, change)?,
        ironsmith_core::Cost::Energy(value) | ironsmith_core::Cost::Mill(value)
        | ironsmith_core::Cost::Life(value) => *value = rewrite_value_words(value, change)?,
        ironsmith_core::Cost::ExileFromHand { color_filter, .. } => {
            if let Some(colors) = color_filter { change.replace_color_words(colors); }
        }
        ironsmith_core::Cost::Effect(effect) => *effect = effect.with_text_change(change)?,
        ironsmith_core::Cost::Mana(_) | ironsmith_core::Cost::Tap | ironsmith_core::Cost::Untap
        | ironsmith_core::Cost::DiscardSource | ironsmith_core::Cost::SacrificeSelf
        | ironsmith_core::Cost::Discard { .. } | ironsmith_core::Cost::DiscardHand
        | ironsmith_core::Cost::RemoveCounters { .. } | ironsmith_core::Cost::AddCounters { .. }
        | ironsmith_core::Cost::RemoveAnyCountersFromSource { .. } | ironsmith_core::Cost::ExileSelf
        | ironsmith_core::Cost::ExileFromGraveyard { .. } | ironsmith_core::Cost::ReturnSelfToHand => {}
    }
    // Core cost equality is structural; nested Effect equality is identity.
    if &rewritten == model { Ok(cost.clone()) }
    else { Cost::from_model(rewritten).map_err(|_| Error::Cost) }
}

fn rewrite_payment_predicate(predicate: &ManaPaymentPredicate, change: TextChange)
    -> Result<ManaPaymentPredicate, Error>
{
    let mut rewritten = predicate.clone();
    match &mut rewritten {
        ManaPaymentPredicate::SourceMatches(filter) => *filter = rewrite_filter_words(filter, change)?,
        ManaPaymentPredicate::All(predicates) | ManaPaymentPredicate::AnyOf(predicates) => {
            *predicates = predicates.iter().map(|predicate| rewrite_payment_predicate(predicate, change))
                .collect::<Result<_, _>>()?;
        }
        ManaPaymentPredicate::Not(predicate) => **predicate = rewrite_payment_predicate(predicate, change)?,
        ManaPaymentPredicate::Any | ManaPaymentPredicate::Purpose(_)
        | ManaPaymentPredicate::GenericManaCost | ManaPaymentPredicate::CostContains(_)
        | ManaPaymentPredicate::CostContainsX | ManaPaymentPredicate::SharesCreatureTypeWithPayersCommander
        | ManaPaymentPredicate::TurnFaceUpMethod(_) | ManaPaymentPredicate::SourceManifested
        | ManaPaymentPredicate::DisturbCost | ManaPaymentPredicate::ActivatedAbilityKeyword(_) => {}
    }
    Ok(rewritten)
}

fn rewrite_mana_restriction(restriction: &ManaUsageRestriction, change: TextChange)
    -> Result<ManaUsageRestriction, Error>
{
    let mut rewritten = restriction.clone();
    match &mut rewritten {
        ManaUsageRestriction::CastSpell { subtype_requirement, .. } => {
            if let Some(ManaUsageSubtypeRequirement::Exact(subtype)) = subtype_requirement {
                change.replace_subtype_word(subtype);
            }
        }
        ManaUsageRestriction::CastSpellMatching { filter, .. }
        | ManaUsageRestriction::CastSpellWithManaBonus { filter, .. }
        | ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp { spell_filter: filter }
        | ManaUsageRestriction::CastSpellOrUnlockDoor { spell_filter: filter } => {
            *filter = rewrite_filter_words(filter, change)?;
        }
        ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching { spell_filter, ability_source_filter } => {
            *spell_filter = rewrite_filter_words(spell_filter, change)?;
            *ability_source_filter = rewrite_filter_words(ability_source_filter, change)?;
        }
        ManaUsageRestriction::PaymentTransaction { restriction, on_spend } => {
            *restriction = restriction.as_ref().map(|predicate| rewrite_payment_predicate(predicate, change)).transpose()?;
            for payload in on_spend {
                payload.predicate = rewrite_payment_predicate(&payload.predicate, change)?;
                payload.effects = rewrite_program_words(&payload.effects, change)?;
                payload.choices = rewrite_choices(&payload.choices, change)?;
            }
        }
        ManaUsageRestriction::ActivateAbility => {}
    }
    Ok(rewritten)
}

fn rewrite_result_predicate(predicate: &EffectPredicate, change: TextChange) -> Result<EffectPredicate, Error> {
    let mut rewritten = predicate.clone();
    match &mut rewritten {
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue { player } => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        EffectPredicate::PriorEffectResult(surface) => surface.filter = rewrite_filter_words(&surface.filter, change)?,
        EffectPredicate::Succeeded | EffectPredicate::Failed | EffectPredicate::Happened
        | EffectPredicate::DidNotHappen | EffectPredicate::SearchedLibrary | EffectPredicate::HappenedNotReplaced
        | EffectPredicate::ExcessDamageDealt | EffectPredicate::DealtDamageToPlayer
        | EffectPredicate::AffectedObjectMatchesCardType { .. } | EffectPredicate::Value(_)
        | EffectPredicate::Chosen | EffectPredicate::WasDeclined
        // Characteristic families and the minimum cohort size contain no
        // authored color, creature-type or land-type word to substitute.
        | EffectPredicate::AffectedObjectsShare { .. } => {}
    }
    Ok(rewritten)
}

/// `None` proves the complete admitted model is unchanged. A changed result
/// intentionally drops the old retained transport model; its native encoder
/// must encode the rewritten typed payload, never reuse stale serialized data.
pub(crate) fn rewrite_effect_words(effect: &Effect, change: TextChange) -> Result<Option<Effect>, Error> {
    if let Some(model) = effect.downcast_ref::<CreateTokenEffect>() {
        let roles = model.text_roles.as_ref().ok_or(Error::TokenDefinition)?;
        if !roles.has_complete_ability_inventory(model.token.abilities.len())
            || roles.colors == ironsmith_core::TokenWordRole::Unrecorded
            || roles.subtypes == ironsmith_core::TokenWordRole::Unrecorded
            || roles.abilities.contains(&ironsmith_core::TokenWordRole::Unrecorded)
            || model.token.spell_effect.is_some() || model.token.aura_attach_filter.is_some()
            || !model.token.alternative_casts.is_empty() || !model.token.optional_costs.is_empty()
            || !model.token.additional_cost.as_all().is_some_and(|costs| costs.is_empty())
        { return Err(Error::TokenDefinition); }
        let mut changed = model.clone();
        changed.count = rewrite_value_words(&model.count, change)?;
        changed.controller = rewrite_player_filter_words(&model.controller, change)?;
        changed.controller_target = model.controller_target.as_ref()
            .map(|target| rewrite_choose_spec_words(target, change)).transpose()?;
        changed.enters_blocking = model.enters_blocking.as_ref()
            .map(|target| rewrite_choose_spec_words(target, change)).transpose()?;
        changed.next_end_step_player = rewrite_player_filter_words(&model.next_end_step_player, change)?;
        if let Some(mode) = &mut changed.attack_target_mode {
            match mode {
                ironsmith_core::CopyAttackTargetMode::Player(player)
                | ironsmith_core::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player) =>
                    *player = rewrite_player_filter_words(player, change)?,
            }
        }
        if roles.colors == ironsmith_core::TokenWordRole::Authored
            && let Some(colors) = &mut changed.token.card.color_indicator
        { change.replace_color_words(colors); }
        if roles.subtypes == ironsmith_core::TokenWordRole::Authored {
            change.replace_subtype_words(&mut changed.token.card.subtypes);
        }
        for (ability, role) in changed.token.abilities.iter_mut().zip(&roles.abilities) {
            if *role == ironsmith_core::TokenWordRole::Authored {
                *ability = rewrite_ability_words(ability, change)?;
            }
        }
        if roles.name == ironsmith_core::TokenNameTextRole::SubtypeDerived {
            changed.token.card.name = ironsmith_core::subtype_derived_token_name(&changed.token.card.subtypes)
                .ok_or(Error::TokenDefinition)?;
        }
        // The immutable template CardId, explicit names, mana symbols, source
        // choices, entry/lifecycle flags, and all link/acquisition metadata
        // remain those of the complete original instruction.
        return Ok(Some(Effect::new(changed)));
    }
    macro_rules! visit {
        ($ty:ty, $model:ident, $body:block) => {
            if let Some(original) = effect.downcast_ref::<$ty>() {
                let mut $model = original.clone();
                $body
                return Ok(($model != *original).then(|| Effect::new($model)));
            }
        };
    }
    macro_rules! value_player {
        ($ty:ty, $field:ident) => { visit!($ty, model, {
            model.$field = rewrite_value_words(&model.$field, change)?;
            model.player = rewrite_player_filter_words(&model.player, change)?;
        }); };
    }
    macro_rules! value_recipient {
        ($ty:ty, $field:ident) => { visit!($ty, model, {
            model.$field = rewrite_value_words(&model.$field, change)?;
            model.player = rewrite_choose_spec_words(&model.player, change)?;
        }); };
    }
    macro_rules! target_only {
        ($ty:ty, $field:ident) => { visit!($ty, model, {
            model.$field = rewrite_choose_spec_words(&model.$field, change)?;
        }); };
    }
    macro_rules! child_list {
        ($ty:ty) => { visit!($ty, model, { model.effects = rewrite_effects(&model.effects, change)?; }); };
    }

    if let Some(model) = effect.downcast_ref::<ApplyContinuousEffect>() {
        let rewritten = super::text_change_modifications::rewrite_apply_continuous_words(model, change)?;
        return Ok(Some(Effect::new(rewritten)));
    }
    visit!(ChangeTextEffect, model, {
        model.duration = super::text_change_modifications::rewrite_until_words(&model.duration, change)?;
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        match &mut model.selection {
            ironsmith_core::TextChangeSelection::CreatureTo(subtype) => change.replace_subtype_word(subtype),
            ironsmith_core::TextChangeSelection::Creature { excluded_new } => change.replace_subtype_words(excluded_new),
            ironsmith_core::TextChangeSelection::Color | ironsmith_core::TextChangeSelection::BasicLand
            | ironsmith_core::TextChangeSelection::ColorOrBasicLand => {}
        }
    });
    value_player!(DrawCardsEffect, count);
    value_player!(MillEffect, count);
    visit!(DiscardHandEffect, model, {
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    value_recipient!(GainLifeEffect, amount);
    visit!(LoseLifeEffect, model, {
        // The current transport model owns a PlayerFilter, not a complete
        // ChooseSpec. Preserve target/count/surface wrappers by refusing
        // those native forms until that transport owner is extended.
        if !matches!(&model.player, crate::target::ChooseSpec::Player(_)) { return Err(Error::Effect); }
        model.amount = rewrite_value_words(&model.amount, change)?;
        model.player = rewrite_choose_spec_words(&model.player, change)?;
    });
    value_recipient!(PayLifeEffect, amount);
    value_recipient!(PayEnergyEffect, amount);
    visit!(DealDamageEffect, model, {
        model.amount = rewrite_value_words(&model.amount, change)?;
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        if let Some(redirect) = &mut model.excess_to_controller {
            redirect.condition = redirect.condition.as_ref().map(|condition| rewrite_condition_words(condition, change)).transpose()?;
        }
    });
    visit!(HealDamageEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.amount = model.amount.as_ref().map(|value| rewrite_value_words(value, change)).transpose()?;
    });
    target_only!(DestroyEffect, spec);
    target_only!(DestroyNoRegenerationEffect, spec);
    target_only!(ExileEffect, spec);
    target_only!(CounterEffect, target);
    target_only!(AttachToEffect, target);
    visit!(AttachObjectsEffect, model, {
        model.objects = rewrite_choose_spec_words(&model.objects, change)?;
        model.target = rewrite_choose_spec_words(&model.target, change)?;
    });
    visit!(TapEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.actor = model.actor.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(UntapEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.actor = model.actor.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(ReturnToHandEffect, model, {
        model.spec = rewrite_choose_spec_words(&model.spec, change)?;
        model.actor_surface = model.actor_surface.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
        model.destination_player_surface = model.destination_player_surface.as_ref()
            .map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(MoveToZoneEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.actor_surface = model.actor_surface.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
        model.destination_player_surface = model.destination_player_surface.as_ref()
            .map(|player| rewrite_player_filter_words(player, change)).transpose()?;
        if let Some(ironsmith_core::LibraryPlacementOrder::ChosenBy(player)) = &mut model.library_order {
            *player = rewrite_player_filter_words(player, change)?;
        }
        if let Some(ironsmith_core::MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player)) = &mut model.attack_target_mode {
            *player = rewrite_player_filter_words(player, change)?;
        }
        for counters in &mut model.enters_with_counters {
            counters.amount = rewrite_value_words(&counters.amount, change)?;
            counters.condition = counters.condition.as_ref().map(|condition| rewrite_condition_words(condition, change)).transpose()?;
            counters.object_filter = counters.object_filter.as_ref().map(|filter| rewrite_filter_words(filter, change)).transpose()?;
        }
    });
    visit!(SacrificeTargetEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.player = model.player.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(SacrificeEffect, model, {
        // This native form additionally carries event tags. Its exact core
        // model presently supports the controller and a fixed quantity only.
        // Dynamic/player-scoped forms use SacrificePlayerEffect below.
        if model.player != crate::target::PlayerFilter::You
            || !matches!(&model.count, crate::effect::Value::Fixed(_))
        { return Err(Error::Effect); }
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.count = rewrite_value_words(&model.count, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    visit!(ironsmith_core::SacrificePlayerEffect, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.count = rewrite_value_words(&model.count, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    visit!(DiscardEffect, model, {
        model.count = rewrite_value_words(&model.count, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
        model.card_filter = model.card_filter.as_ref().map(|filter| rewrite_filter_words(filter, change)).transpose()?;
    });
    visit!(PutCountersEffect, model, {
        model.amount = rewrite_value_words(&model.amount, change)?;
        model.target = rewrite_choose_spec_words(&model.target, change)?;
    });
    visit!(RemoveCountersEffect, model, {
        model.count = rewrite_value_words(&model.count, change)?;
        model.target = rewrite_choose_spec_words(&model.target, change)?;
    });
    target_only!(DoubleCountersEffect, target);
    visit!(MoveAllCountersEffect, model, {
        model.from = rewrite_choose_spec_words(&model.from, change)?;
        model.to = rewrite_choose_spec_words(&model.to, change)?;
    });
    visit!(RevealFromHandEffect, model, {
        model.count = rewrite_value_words(&model.count, change)?;
        if let Some(colors) = &mut model.color_filter { change.replace_color_words(colors); }
    });

    visit!(AddManaEffect, model, {
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    value_player!(AddScaledManaEffect, amount);
    value_player!(AddColorlessManaEffect, amount);
    value_player!(AddManaOfAnyOneColorEffect, amount);
    value_player!(AddManaFromCommanderColorIdentityEffect, amount);
    visit!(AddManaOfChosenColorEffect, model, {
        model.amount = rewrite_value_words(&model.amount, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
        // fixed_option represents the alternative printed mana symbol in
        // "{R} or one mana of the chosen color". The chosen-color reference
        // is read at execution; neither is an authored color word.
    });
    visit!(AddManaOfAnyColorEffect, model, {
        model.amount = rewrite_value_words(&model.amount, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
        // A restricted list currently lacks provenance distinguishing a word
        // restriction from enumerated mana symbols. Never guess its origin.
        if let Some(colors) = &model.available_colors {
            for color in colors {
                let mut rewritten = *color;
                change.replace_color_word(&mut rewritten);
                if rewritten != *color { return Err(Error::Effect); }
            }
        }
    });
    visit!(AddManaOfColorsAmongEffect, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    visit!(AddOneManaOfAnyColorAmongEffect, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    visit!(PayManaEffect, model, {
        model.player = rewrite_choose_spec_words(&model.player, change)?;
        model.x_value = model.x_value.as_ref().map(|value| rewrite_value_words(value, change)).transpose()?;
        model.x_maximum = model.x_maximum.as_ref().map(|value| rewrite_value_words(value, change)).transpose()?;
    });

    visit!(TargetOnlyEffect, model, {
        model.target = rewrite_choose_spec_words(&model.target, change)?;
        model.chooser = model.chooser.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(ChooseObjectsEffect, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.count_value = model.count_value.as_ref().map(|value| rewrite_value_words(value, change)).transpose()?;
        model.aggregate_constraint = model.aggregate_constraint.as_ref()
            .map(|constraint| rewrite_aggregate_constraint_words(constraint, change)).transpose()?;
        model.chooser = rewrite_player_filter_words(&model.chooser, change)?;
    });
    visit!(ChoosePlayerEffect, model, {
        model.chooser = rewrite_player_filter_words(&model.chooser, change)?;
        model.filter = rewrite_player_filter_words(&model.filter, change)?;
    });
    visit!(TagMatchingObjectsEffect, model, { model.filter = rewrite_filter_words(&model.filter, change)?; });

    child_list!(SequenceEffect);
    child_list!(ManaRetainedEffect);
    child_list!(ForEachTaggedEffect);
    child_list!(ForEachControllerOfTaggedEffect);
    child_list!(ForEachTaggedPlayerEffect);
    visit!(WithIdEffect, model, { model.effect = Box::new(model.effect.with_text_change(change)?); });
    visit!(TaggedEffect, model, { model.effect = Box::new(model.effect.with_text_change(change)?); });
    visit!(ExecuteWithSourceEffect, model, {
        model.source = rewrite_choose_spec_words(&model.source, change)?;
        model.effect = Box::new(model.effect.with_text_change(change)?);
    });
    visit!(MayEffect, model, {
        // The shared optional-action model fixes this policy to Decline.
        if model.fallback != crate::decision::FallbackStrategy::Decline { return Err(Error::Effect); }
        model.effects = rewrite_effects(&model.effects, change)?;
        model.decider = model.decider.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(UnlessActionEffect, model, {
        model.effects = rewrite_effects(&model.effects, change)?;
        model.alternative = rewrite_effects(&model.alternative, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
    });
    // This wrapper contains runtime Cost objects: avoid its derived equality,
    // which would indirectly consult display strings through Cost::PartialEq.
    if let Some(original) = effect.downcast_ref::<UnlessPaysEffect>() {
        let mut model = original.clone();
        model.effects = rewrite_effects(&model.effects, change)?;
        model.player = rewrite_player_filter_words(&model.player, change)?;
        model.cost = rewrite_total_cost_words(&model.cost, change)?;
        let changed = model.effects != original.effects || model.player != original.player
            || !same_total_cost_identity(&model.cost, &original.cost);
        return Ok(changed.then(|| Effect::new(model)));
    }
    visit!(ConditionalEffect, model, {
        model.condition = rewrite_condition_words(&model.condition, change)?;
        model.if_true = rewrite_effects(&model.if_true, change)?;
        model.if_false = rewrite_effects(&model.if_false, change)?;
    });
    visit!(IfEffect, model, {
        model.predicate = rewrite_result_predicate(&model.predicate, change)?;
        model.then = rewrite_effects(&model.then, change)?;
        model.else_ = rewrite_effects(&model.else_, change)?;
    });
    visit!(ReflexiveTriggerEffect, model, {
        model.predicate = rewrite_result_predicate(&model.predicate, change)?;
        model.effects = rewrite_effects(&model.effects, change)?;
        model.choices = rewrite_choices(&model.choices, change)?;
        model.intervening_if = model.intervening_if.as_ref().map(|condition| rewrite_condition_words(condition, change)).transpose()?;
    });
    visit!(ChooseModeEffect, model, {
        for mode in &mut model.modes { mode.effects = rewrite_effects(&mode.effects, change)?; }
        model.common_prefix_effects = rewrite_effects(&model.common_prefix_effects, change)?;
        model.chooser = model.chooser.as_ref().map(|player| rewrite_player_filter_words(player, change)).transpose()?;
        for value in [&mut model.min, &mut model.max, &mut model.choose_count, &mut model.min_choose_count] {
            *value = rewrite_value_words(value, change)?;
        }
        if let Some(range) = &mut model.conditional_mode_range {
            range.min_modes = rewrite_value_words(&range.min_modes, change)?;
            range.max_modes = rewrite_value_words(&range.max_modes, change)?;
        }
    });
    visit!(ForEachObject, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.effects = rewrite_effects(&model.effects, change)?;
    });
    visit!(ForEachObjectCorrelatedResultEffect, model, {
        model.filter = rewrite_filter_words(&model.filter, change)?;
        model.producer_effects = rewrite_effects(&model.producer_effects, change)?;
        model.consumer_effects = rewrite_effects(&model.consumer_effects, change)?;
    });
    visit!(ForPlayersEffect, model, {
        model.filter = rewrite_player_filter_words(&model.filter, change)?;
        model.effects = rewrite_effects(&model.effects, change)?;
    });
    visit!(RepeatEffectsEffect, model, {
        model.count = rewrite_value_words(&model.count, change)?;
        model.effects = rewrite_effects(&model.effects, change)?;
    });
    visit!(RepeatProcessEffect, model, {
        model.predicate = rewrite_result_predicate(&model.predicate, change)?;
        model.effects = rewrite_effects(&model.effects, change)?;
    });
    visit!(RepeatProcessPromptEffect, model, {
        model.decider = model.decider.as_ref()
            .map(|player| rewrite_player_filter_words(player, change)).transpose()?;
    });
    visit!(ManaRestrictedEffect, model, {
        model.effects = rewrite_effects(&model.effects, change)?;
        model.restrictions = model.restrictions.iter().map(|restriction| rewrite_mana_restriction(restriction, change))
            .collect::<Result<_, _>>()?;
    });
    visit!(CumulativeUpkeepEffect, model, {
        model.player = rewrite_player_filter_words(&model.player, change)?;
        model.payment = rewrite_effects(&model.payment, change)?;
        model.failure = rewrite_effects(&model.failure, change)?;
    });

    // These complete native payloads consist solely of non-word identities,
    // counter kinds, source references or wordless rules operations.
    if effect.downcast_ref::<CipherEffect>().is_some()
        || effect.downcast_ref::<NoteLifeTotalEffect>().is_some()
        || effect.downcast_ref::<RevealTaggedEffect>().is_some()
        || effect.downcast_ref::<RevealSourceFromHandEffect>().is_some()
        || effect.downcast_ref::<TagAttachedToSourceEffect>().is_some()
    { return Ok(None); }
    Err(Error::Effect)
}

#[cfg(test)]
mod tests {
    // Source-authored regressions, intentionally unrun under the user's
    // campaign-wide prohibition on builds, compiler probes and test runs.
    use super::*;
    use crate::effect::{Condition, EffectId, Value};
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use ironsmith_core::{Color, ColorSet, ManaCost, ManaSymbol, Subtype};

    fn change() -> TextChange { TextChange::color(Color::Black, Color::Blue).unwrap() }
    fn black() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLACK), ..ObjectFilter::default() } }
    fn blue() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() } }
    fn word_effect() -> Effect { Effect::new(DrawCardsEffect::you(Value::Count(black()))) }

    #[test]
    fn self_replacement_conditions_and_flattened_program_keep_their_declared_owner() {
        let unchanged = Effect::draw(1);
        let changed = word_effect();
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([17; 32]), pair: 29,
        };
        let program = ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![unchanged.clone()], starts_new_source_line: true,
            self_replacements: vec![crate::resolution::SelfReplacementBranch {
                condition: Condition::YouControl(black()), replacement_effects: vec![changed.clone()],
                presentation_label: None, condition_after_replacement: true,
                leading_instead_surface: true, starts_new_source_line: true,
            }],
        }]).with_linked_exile_pair(pair);
        let rewritten = rewrite_program_words(&program, change()).unwrap();
        assert_eq!(rewritten.linked_exile_pair, Some(pair));
        assert_eq!(rewritten.flattened_default_effects(), &[unchanged]);
        assert!(rewritten.segments[0].starts_new_source_line);
        let branch = &rewritten.segments[0].self_replacements[0];
        assert_eq!(branch.condition, Condition::YouControl(blue()));
        assert!(branch.condition_after_replacement && branch.leading_instead_surface && branch.starts_new_source_line);
        assert_eq!(branch.replacement_effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert_eq!(program.segments[0].self_replacements[0].condition, Condition::YouControl(black()));
        assert_eq!(program.segments[0].self_replacements[0].replacement_effects, vec![changed]);
    }

    #[test]
    fn nested_immutable_rewrites_preserve_result_tags_and_cached_identity() {
        let original = Effect::new(TaggedEffect::new("black-receipt", Effect::new(WithIdEffect::new(
            EffectId(41), word_effect(),
        )))).with_serialized_model("stale transport must not be carried forward");
        let captured = original.clone();
        let first = original.with_text_change(change()).unwrap();
        let second = original.with_text_change(change()).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, original);
        assert!(first.serialized_model().is_none());
        assert_eq!(original, captured);
        let tagged = first.downcast_ref::<TaggedEffect>().unwrap();
        assert_eq!(tagged.tag.as_str(), "black-receipt");
        let observed = tagged.effect.downcast_ref::<WithIdEffect>().unwrap();
        assert_eq!(observed.id, EffectId(41));
        assert_eq!(observed.effect.downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert_eq!(original.downcast_ref::<TaggedEffect>().unwrap().effect
            .downcast_ref::<WithIdEffect>().unwrap().effect.downcast_ref::<DrawCardsEffect>().unwrap().count,
            Value::Count(black()));
    }

    #[test]
    fn activated_cost_choice_and_restriction_words_change_without_changing_mana_symbols() {
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Black]]);
        let mut ability = Ability::activated(TotalCost::one_of(vec![
            TotalCost::from_cost(Cost::sacrifice(black())),
            TotalCost::from_cost(Cost::dynamic_mana(ironsmith_core::DynamicManaCost::from_x(
                mana.clone(), Value::Count(black()),
            ))),
        ]), vec![word_effect()]);
        let AbilityKind::Activated(model) = &mut ability.kind else { unreachable!() };
        model.choices.push(ChooseSpec::target(ChooseSpec::Object(black())));
        model.activation_condition = Some(Condition::YouControl(black()));
        model.activation_restrictions.push(Condition::YouControl(black()));
        model.mana_output = Some(vec![ManaSymbol::Black]);
        model.mana_usage_restrictions.push(ManaUsageRestriction::CastSpellMatching {
            filter: black(), restrict_to_matching_spell: true, grant_uncounterable: true,
            enters_with_counters: vec![], granted_abilities: vec![],
        });
        let rewritten = rewrite_ability_words(&ability, change()).unwrap();
        assert_eq!(rewritten.functional_zones, ability.functional_zones);
        let AbilityKind::Activated(model) = &rewritten.kind else { unreachable!() };
        assert_eq!(model.choices, vec![ChooseSpec::target(ChooseSpec::Object(blue()))]);
        assert_eq!(model.activation_condition, Some(Condition::YouControl(blue())));
        assert_eq!(model.activation_restrictions, vec![Condition::YouControl(blue())]);
        assert_eq!(model.mana_output, Some(vec![ManaSymbol::Black]));
        let branches = model.mana_cost.as_one_of().unwrap();
        assert_eq!(branches[0].costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue())));
        let ironsmith_core::Cost::DynamicMana(dynamic) = branches[1].costs()[0].compiled_model().unwrap() else { unreachable!() };
        assert_eq!(dynamic.base, mana);
        assert_eq!(dynamic.x_value, Some(Value::Count(blue())));
        let ManaUsageRestriction::CastSpellMatching { filter, .. } = &model.mana_usage_restrictions[0] else { unreachable!() };
        assert_eq!(filter, &blue());
    }

    #[test]
    fn optional_payment_detects_changed_cost_without_using_rendered_cost_equality() {
        let original = Effect::new(UnlessPaysEffect::new_total_cost(vec![Effect::draw(1)],
            PlayerFilter::You, TotalCost::from_cost(Cost::sacrifice(black()))));
        let changed = original.with_text_change(change()).unwrap();
        assert_ne!(changed, original);
        let model = changed.downcast_ref::<UnlessPaysEffect>().unwrap();
        assert_eq!(model.cost.costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue())));
        assert_eq!(changed, original.with_text_change(change()).unwrap());
    }

    #[test]
    fn mode_metadata_and_mana_pips_are_preserved_while_quantities_and_instructions_change() {
        let mut mode = ChooseModeEffect::new(vec![ironsmith_core::EffectMode::new(
            "Black Knight is a name in this label", vec![word_effect()],
        )], Value::Fixed(1), Value::Count(black()), true);
        mode.common_prefix_effects.push(Effect::draw(1));
        mode.mode_additional_mana_costs.push(ManaCost::from_pips(vec![vec![ManaSymbol::Black]]));
        mode.disallow_previously_chosen_modes_this_turn = true;
        mode.conditional_mode_range = Some(ironsmith_core::ConditionalModeRange::new(
            "black-optional-cost", Value::Fixed(1), Value::Count(black()),
        ));
        let original = Effect::new(mode.clone());
        let changed = original.with_text_change(change()).unwrap();
        let model = changed.downcast_ref::<ChooseModeEffect>().unwrap();
        assert_eq!(model.max, Value::Count(blue()));
        assert_eq!(model.modes[0].source_text, mode.modes[0].source_text);
        assert_eq!(model.mode_additional_mana_costs, mode.mode_additional_mana_costs);
        assert_eq!(model.common_prefix_effects, mode.common_prefix_effects);
        assert!(model.disallow_previously_chosen_modes_this_turn);
        assert_eq!(model.conditional_mode_range.as_ref().unwrap().required_optional_cost,
            mode.conditional_mode_range.as_ref().unwrap().required_optional_cost);
        assert_eq!(model.conditional_mode_range.as_ref().unwrap().max_modes, Value::Count(blue()));
    }

    #[test]
    fn chosen_references_fixed_mana_symbols_and_absent_words_keep_original_definitions() {
        let source = Effect::new(AddManaOfChosenColorEffect::with_fixed_option(1, PlayerFilter::ChosenPlayer, Color::Black));
        assert_eq!(source.with_text_change(change()).unwrap(), source);
        let source = Effect::new(SequenceEffect::new(vec![word_effect(), Effect::new(AddManaEffect::you(vec![ManaSymbol::Black]))]));
        let no_occurrence = TextChange::creature_type(Subtype::Elf, Subtype::Vampire).unwrap();
        assert_eq!(source.with_text_change(no_occurrence).unwrap(), source);
    }

    #[test]
    fn attachment_programs_rewrite_object_and_player_predicates_preserving_bound_recipients() {
        let source = Effect::new(AttachToEffect::new(ChooseSpec::target(ChooseSpec::Object(black()))));
        let changed = source.with_text_change(change()).unwrap();
        assert_eq!(changed.downcast_ref::<AttachToEffect>().unwrap().target,
            ChooseSpec::target(ChooseSpec::Object(blue())));
        let source = Effect::new(AttachObjectsEffect::new(ChooseSpec::All(black()),
            ChooseSpec::target(ChooseSpec::Player(PlayerFilter::ControlsMost { filter: Box::new(black()) })))
            .with_individual_targets());
        let changed = source.with_text_change(change()).unwrap();
        let model = changed.downcast_ref::<AttachObjectsEffect>().unwrap();
        assert_eq!(model.objects, ChooseSpec::All(blue()));
        assert_eq!(model.target, ChooseSpec::target(ChooseSpec::Player(
            PlayerFilter::ControlsMost { filter: Box::new(blue()) })));
        assert!(model.individual_targets);
        let captured = Effect::new(AttachObjectsEffect::new(ChooseSpec::Tagged("black-attachments".into()),
            ChooseSpec::SpecificPlayer(crate::ids::PlayerId::from_index(1))).with_individual_targets());
        assert_eq!(captured.with_text_change(change()).unwrap(), captured);
    }

    #[derive(Debug, Clone)]
    struct UnknownNative;
    impl EffectExecutor for UnknownNative {
        fn execute(&self, _: &mut crate::game_state::GameState, _: &mut ExecutionContext)
            -> Result<crate::effect::EffectOutcome, ExecutionError>
        { Ok(crate::effect::EffectOutcome::resolved()) }
    }

    #[test]
    fn unsupported_child_rejects_the_whole_definition_without_publishing_a_partial_program() {
        let first = word_effect();
        let unknown = Effect::new(UnknownNative);
        let program = ResolutionProgram::from_effects(vec![first.clone(), unknown.clone()]);
        assert!(matches!(rewrite_program_words(&program, change()), Err(Error::Effect)));
        assert_eq!(program.flattened_default_effects(), &[first.clone(), unknown]);
        assert_eq!(first.downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(black()));
        let mut ability = Ability::activated(TotalCost::free(), vec![first]);
        let AbilityKind::Activated(model) = &mut ability.kind else { unreachable!() };
        model.additional_restrictions.push("opaque legacy restriction".into());
        assert!(matches!(rewrite_ability_words(&ability, change()), Err(Error::ActivatedAbility)));
    }

    #[test]
    fn incomplete_native_transport_domains_are_explicit_holds() {
        let target_life = Effect::new(LoseLifeEffect::new(Value::Count(black()), ChooseSpec::target_player()));
        assert!(matches!(target_life.with_text_change(change()), Err(Error::Effect)));
        let optional = Effect::new(MayEffect::new(vec![word_effect()]).with_fallback(crate::decision::FallbackStrategy::Accept));
        assert!(matches!(optional.with_text_change(change()), Err(Error::Effect)));
        let dynamic_tagged_sacrifice = Effect::new(SacrificeEffect::you(black(), Value::Count(black())));
        assert!(matches!(dynamic_tagged_sacrifice.with_text_change(change()), Err(Error::Effect)));
        let chosen_mana = Effect::new(AddManaOfAnyColorEffect::you_restricted(1, vec![Color::Black, Color::Blue]));
        assert!(matches!(chosen_mana.with_text_change(change()), Err(Error::Effect)));
    }
    #[test]
    fn repeat_program_words_keep_receipt_identity_condition_capture_and_explicit_chooser() {
        let shared = EffectPredicate::AffectedObjectsShare {
            required_count: 2, characteristic: ironsmith_core::ObjectCharacteristic::Name,
        };
        let gate = Effect::new(ConditionalEffect::if_only(Condition::YouControl(black()),
            vec![word_effect()]).with_condition_result(true));
        let prompt = Effect::new(RepeatProcessPromptEffect::new(
            ironsmith_core::RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes,
        ).with_decider(Some(PlayerFilter::ControlsMost { filter: Box::new(black()) })));
        let original = Effect::new(RepeatProcessEffect::new(
            vec![gate, prompt], EffectId(73), shared.clone(),
        ));
        let rewritten = original.with_text_change(change()).unwrap();
        let repeat = rewritten.downcast_ref::<RepeatProcessEffect>().unwrap();
        assert_eq!(repeat.condition, EffectId(73));
        assert_eq!(repeat.predicate, shared);
        let gate = repeat.effects[0].downcast_ref::<ConditionalEffect>().unwrap();
        assert!(gate.capture_condition_result);
        assert_eq!(gate.condition, Condition::YouControl(blue()));
        assert_eq!(gate.if_true[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        let prompt = repeat.effects[1].downcast_ref::<RepeatProcessPromptEffect>().unwrap();
        assert_eq!(prompt.decider, Some(PlayerFilter::ControlsMost { filter: Box::new(blue()) }));
        let unchanged = original.downcast_ref::<RepeatProcessEffect>().unwrap();
        assert_eq!(unchanged.effects[0].downcast_ref::<ConditionalEffect>().unwrap().condition,
            Condition::YouControl(black()));
    }

}

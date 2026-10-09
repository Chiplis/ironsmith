//! Copy spell effect implementation.
#[cfg(test)]
use crate::events::BecomesTargetedEvent;

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::spells::SpellCopiedEvent;
use crate::game_state::{GameState, StackEntry, Target};
use crate::object::Object;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Effect that copies a spell on the stack.
///
/// Per Rule 707.10, when a spell is copied:
/// - The copy has the same characteristics and choices (modes, targets, X value)
/// - The copy is controlled by the player who copied it
/// - The copy is put on the stack above the original
///
/// # Fields
///
/// * `target` - The target specification for the spell to copy
/// * `count` - How many copies to create
///
/// # Example
///
/// ```ignore
/// // Copy target instant or sorcery spell
/// let effect = CopySpellEffect::new(ChooseSpec::spell(), 1);
/// ```
pub type CopySpellEffect = ironsmith_core::CopySpellEffect;

fn target_from_resolved_target(target: &ResolvedTarget) -> Target {
    match target {
        ResolvedTarget::Object(id) => Target::Object(*id),
        ResolvedTarget::Player(id) => Target::Player(*id),
    }
}

pub(crate) fn resolving_source_stack_entry(ctx: &ExecutionContext) -> StackEntry {
    let mut entry = StackEntry::new(ctx.source, ctx.controller);
    entry.provenance = ctx.provenance;
    entry.linked_exile_owner = ctx.linked_exile_owner.clone();
    entry.source_number_owner = ctx.source_number_owner.clone();
    entry.activation_origin = ctx.activation_origin.clone();
    entry.activation_definition = ctx.activation_definition;
    entry.ability_index = ctx.ability_index;
    entry.targets = ctx
        .targets
        .iter()
        .map(target_from_resolved_target)
        .collect();
    entry.target_assignments = ctx.target_assignments.clone();
    entry.target_distributions = ctx.target_distributions.clone();
    entry.iteration = ctx.iteration;
    entry.x_value = ctx.x_value;
    entry.mana_spent_on_activation = ctx.mana.activation_payment.clone();
    entry.ninjutsu_attack_target = ctx.ninjutsu_attack_target.clone();
    entry.casting_method = ctx.casting_method.clone();
    entry.optional_costs_paid = ctx.optional_costs_paid.clone();
    entry.defending_player = ctx.combat.defending_player;
    entry.defending_player_reference = ctx.combat.defending_player_reference;
    entry.chosen_player = ctx.combat.chosen_player;
    entry.source_snapshot = ctx.source_snapshot.clone();
    entry.triggering_event = ctx.triggering_event.clone();
    entry.event_value_amount = ctx.event_value_amount;
    entry.chosen_modes = ctx.chosen_modes.clone();
    entry.tagged_objects = ctx.tagged_objects.clone();
    entry.effect_outcomes = ctx.effect_outcomes.clone();
    entry
}

pub(crate) fn stack_entry_for_copy_target(
    game: &GameState,
    target_id: crate::ids::ObjectId,
    ctx: &ExecutionContext,
    kind: Option<crate::filter::StackObjectKind>,
) -> Result<Option<StackEntry>, ExecutionError> {
    // An ability named by its own stack id.
    if let Some(entry) = game.stack_ability_entry(target_id) {
        return Ok(Some(entry.clone()));
    }
    if let Some(activation) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
        && activation.source == target_id
        && let Some(provenance) = activation.stack_entry_provenance
    {
        // Several activations can share one permanent's object ID. The event
        // identifies the activation that triggered this copy, including when
        // that entry has since left the stack and no copy can be made.
        return Ok(game
            .stack
            .iter()
            .find(|entry| {
                entry.is_ability && entry.object_id == target_id && entry.provenance == provenance
            })
            .cloned());
    }
    if let Some(triggered) = ctx
        .triggering_event
        .as_ref()
        .and_then(|event| event.downcast::<crate::events::AbilityTriggeredEvent>())
        && triggered.source == target_id
    {
        // "copy that ability" after "... causes a triggered ability of that
        // creature to trigger" (CR 707.10): the event names the exact
        // triggered ability by its structural identity.
        return Ok(game
            .stack
            .iter()
            .rev()
            .find(|entry| {
                entry.is_ability
                    && entry.object_id == target_id
                    && entry.trigger_identity == Some(triggered.trigger_identity)
            })
            .cloned());
    }
    // Abilities share their source's object ID (a storm trigger has its
    // spell's ID): prefer the most recent entry of the targeted kind.
    let of_kind = kind.and_then(|kind| {
        game.stack.iter().rev().find(|e| {
            e.object_id == target_id
                && <crate::filter::ObjectFilter as crate::filter::ObjectFilterExt>::stack_entry_matches_kind(e, kind)
        })
    });
    if let Some(entry) = of_kind
        .or_else(|| game.stack.iter().find(|e| e.object_id == target_id))
        .cloned()
    {
        if game
            .object(target_id)
            .is_none_or(|obj| obj.zone != Zone::Stack && !entry.is_ability)
        {
            return Ok(None);
        }
        return Ok(Some(entry));
    }

    let target_obj = game
        .object(target_id)
        .ok_or(ExecutionError::ObjectNotFound(target_id))?;
    if target_id == ctx.source && target_obj.zone == Zone::Stack {
        return Ok(Some(resolving_source_stack_entry(ctx)));
    }

    Ok(None)
}

/// Last known information for the resolving ability's source spell after it
/// left the stack. Storm, casualty, replicate, conspire and demonstrate copy
/// "it" even if the original was countered in response (CR 702.40a rulings;
/// CR 608.2h): the copy uses the spell as it last existed on the stack.
pub(crate) fn departed_source_spell_lki(
    game: &GameState,
    ctx: &ExecutionContext,
    target_id: crate::ids::ObjectId,
) -> Option<(Object, StackEntry)> {
    if target_id != ctx.source
        || game
            .object(target_id)
            .is_some_and(|object| object.zone == Zone::Stack)
        || game
            .stack
            .iter()
            .any(|entry| !entry.is_ability && entry.object_id == target_id)
    {
        return None;
    }
    let lki = game.turn_store.cast_spell_lki.get(&target_id)?;
    let (object, entry) = lki.as_ref();
    let mut object = object.clone();
    object.zone = Zone::Stack;
    Some((object, entry.clone()))
}

pub(crate) fn create_stack_copy_from_object(
    game: &mut GameState,
    source: &Object,
    _target_id: crate::ids::ObjectId,
    original_entry: &StackEntry,
    copier: crate::ids::PlayerId,
    removed_supertypes: &[crate::types::Supertype],
    mut customize_copy: impl FnMut(&mut Object),
    targets_override: Option<Vec<Target>>,
) -> Result<Option<crate::ids::ObjectId>, ExecutionError> {
    // CR 101.2: copying is prohibited even when another instruction offers it.
    // A copied ability is independent of the spell's prohibition. A departed
    // spell is evaluated from its captured stack incarnation, never a newer
    // card with the same stable identity.
    if !original_entry.is_ability {
        let abilities = if game
            .object(source.id)
            .is_some_and(|object| object.zone == Zone::Stack)
        {
            game.current_abilities(source.id)
                .unwrap_or_else(|| source.abilities_vec())
        } else {
            source.abilities_vec()
        };
        if abilities.iter().any(|ability| {
            matches!(&ability.kind,
            crate::ability::AbilityKind::Static(rule)
                if rule.id() == crate::static_abilities::StaticAbilityId::CantBeCopied)
        }) {
            return Ok(None);
        }
    }
    // A spell copy takes the source's frozen layer-1 definition, including
    // earlier copy effects. Layer-3 word substitutions remain uncopiable.
    // Ability copies keep their independently captured StackEntry program.
    let copied_values = if original_entry.is_ability { None } else {
        let values = if game.object(source.id).is_some_and(|object| object.zone == Zone::Stack) {
            let view = game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
            let effects = view.all_continuous_effects();
            crate::continuous::copiable_values_with_effects(source.id, view.objects_map(), &effects,
                &view.battlefield, view.commander_objects(), &view)
                .ok_or_else(|| ExecutionError::IncompleteEvidence("copied spell has no layer-one definition".into()))?
        } else {
            crate::snapshot::CopiableValues::from_spell_object(source)
        };
        if !values.spell_effect.has_complete_definition() {
            return Err(ExecutionError::ContinuousDiscovery(
                crate::static_ability_processor::StaticEffectDiscoveryError::TextChangeDomain(
                    crate::continuous::text_changes::TextChangeDomainError::SpellProgram)));
        }
        Some(values)
    };
    let copy_id = game.new_object_id();
    let mut copy_obj = Object::spell_copy_of(source, copy_id, copier);
    if let Some(values) = &copied_values { copy_obj.copy_spell_values_from_values(values); }
    if !removed_supertypes.is_empty() {
        copy_obj
            .supertypes
            .retain(|supertype| !removed_supertypes.contains(supertype));
    }
    customize_copy(&mut copy_obj);
    copy_obj.zone = Zone::Stack;
    let mut copy_entry = StackEntry::new(copy_id, copier);
    copy_entry.provenance = original_entry.provenance;
    copy_entry.targets = targets_override.unwrap_or_else(|| original_entry.targets.clone());
    copy_entry.target_assignments = original_entry.target_assignments.clone();
    copy_entry.target_distributions = original_entry.target_distributions.clone();
    copy_entry.x_value = original_entry.x_value;
    copy_entry.activation_cost_has_x = original_entry.activation_cost_has_x;
    copy_entry.activation_cost_has_tap = original_entry.activation_cost_has_tap;
    // CR 707.10: mana isn't copiable, so a copied ability has no mana spent
    // to activate it either.
    copy_entry.mana_spent_on_activation = crate::player::ManaPool::default();
    copy_entry.ability_effects = original_entry.ability_effects.clone();
    copy_entry.linked_exile_owner = original_entry.linked_exile_owner.clone();
    copy_entry.source_number_owner = original_entry.source_number_owner.clone();
    copy_entry.ninjutsu_attack_target = original_entry.ninjutsu_attack_target.clone();
    copy_entry.is_ability = original_entry.is_ability;
    copy_entry.casting_method = original_entry.casting_method.clone();
    copy_entry.optional_costs_paid = original_entry.optional_costs_paid.clone();
    if !copy_entry.is_ability {
        // Ability copies continue to retain their source's casting receipt.
        copy_entry.optional_costs_paid.clear_uncopied_cast_facts();
    }
    copy_entry.defending_player = original_entry.defending_player;
    copy_entry.defending_player_reference = original_entry.defending_player_reference;
    copy_entry.chosen_player = original_entry.chosen_player;
    copy_entry.source_snapshot = original_entry.source_snapshot.clone();
    copy_entry.source_name = original_entry.source_name.clone();
    copy_entry.source_stable_id = original_entry.source_stable_id;
    copy_entry.chosen_modes = original_entry.chosen_modes.clone();
    copy_entry.spliced_cards = original_entry.spliced_cards.clone();
    copy_entry.keyword_payment_contributions = original_entry.keyword_payment_contributions.clone();
    copy_entry.crew_contributors = original_entry.crew_contributors.clone();
    copy_entry.saddle_contributors = original_entry.saddle_contributors.clone();
    copy_entry.iteration = original_entry.iteration;
    copy_entry.tagged_objects = original_entry.tagged_objects.clone();
    copy_entry.effect_outcomes = original_entry.effect_outcomes.clone();
    // A copy of a triggered ability refers to the same trigger event as the
    // original ("that much", "that player", intervening-if recheck), and a
    // copy of an activated ability keeps its activation context (CR 707.10,
    // 603.4, 603.7c).
    copy_entry.triggering_event = original_entry.triggering_event.clone();
    copy_entry.event_value_amount = original_entry.event_value_amount;
    copy_entry.trigger_identity = original_entry.trigger_identity;
    copy_entry.ability_index = original_entry.ability_index;
    copy_entry.activation_origin = original_entry.activation_origin.clone();
    copy_entry.activation_definition = original_entry.activation_definition;
    copy_entry.intervening_if = original_entry.intervening_if.clone();
    copy_entry.mana_usage_restrictions = original_entry.mana_usage_restrictions.clone();
    copy_entry.mana_source_chosen_creature_type = original_entry.mana_source_chosen_creature_type;
    if !copy_entry.remap_target_distributions(&original_entry.targets) {
        return Err(ExecutionError::InvalidTarget);
    }

    let announced_type = game.chosen_subtype(source.id).filter(|_| {
        source.spell_effect.as_ref().is_some_and(|program| {
            crate::game_loop::spell_program_uses_chosen_creature_type_target(
                game,
                program,
                original_entry.controller,
                Some(source.id),
                original_entry.chosen_modes.as_deref(),
            )
        })
    });
    game.add_object(copy_obj);
    if let Some(subtype) = announced_type {
        game.set_chosen_subtype(copy_id, subtype);
    }

    if let Some(chosen_player) = copy_entry.chosen_player {
        game.set_chosen_player(copy_id, chosen_player);
    }

    game.stack.push(copy_entry);
    Ok(Some(copy_id))
}

/// Remove the stack object that stood in for a copy of an ability once that
/// copy leaves the stack.
///
/// A copied ability is represented by a spell-copy object in the stack zone
/// (so it can be retargeted and named like any stack object). Unlike a spell
/// copy it never moves to another zone, so CR 704.5e never sees it: without
/// this the object would linger for the rest of the game. The object is kept
/// while any stack entry still uses it (an ability of a spell copy, such as
/// a copied Lightning Storm, uses the spell copy itself).
pub(crate) fn discard_departed_ability_copy_object(game: &mut GameState, entry: &StackEntry) {
    if !entry.is_ability
        || entry
            .source_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.object_id == entry.object_id)
        || game
            .stack
            .iter()
            .any(|other| other.object_id == entry.object_id)
    {
        return;
    }
    if game.object(entry.object_id).is_some_and(|object| {
        object.kind == crate::object::ObjectKind::SpellCopy && object.zone == Zone::Stack
    }) {
        game.remove_object(entry.object_id);
    }
}

pub(crate) fn create_stack_copy(
    game: &mut GameState,
    target_id: crate::ids::ObjectId,
    original_entry: &StackEntry,
    copier: crate::ids::PlayerId,
    removed_supertypes: &[crate::types::Supertype],
    targets_override: Option<Vec<Target>>,
) -> Result<Option<crate::ids::ObjectId>, ExecutionError> {
    let target = game
        .object(target_id)
        .ok_or(ExecutionError::ObjectNotFound(target_id))?
        .clone();
    create_stack_copy_from_object(
        game,
        &target,
        target_id,
        original_entry,
        copier,
        removed_supertypes,
        |_| {},
        targets_override,
    )
}

/// A copy that targets a player or object makes it become the target of the
/// copy (ward, "becomes the target" triggers). Each distinct target becomes a
/// target once (CR 115.3).
fn queue_copy_becomes_targeted_events(
    game: &mut GameState,
    ctx: &ExecutionContext,
    copy_id: crate::ids::ObjectId,
) {
    let Some(entry) = game
        .stack
        .iter()
        .find(|entry| entry.object_id == copy_id)
        .cloned()
    else {
        return;
    };
    let mut targeted_seen = Vec::new();
    for target in &entry.targets {
        if targeted_seen.contains(target) {
            continue;
        }
        targeted_seen.push(*target);
        game.queue_trigger_event(
            ctx.provenance,
            TriggerEvent::new_with_provenance(
                crate::events::BecomesTargetedEvent::from_stack_entry(*target, &entry),
                ctx.provenance,
            ),
        );
    }
}

trait CopyCharacteristicModifiers {
    fn apply_copy_characteristic_modifiers(&self, copy: &mut Object);
}

impl CopyCharacteristicModifiers for CopySpellEffect {
    fn apply_copy_characteristic_modifiers(&self, copy: &mut Object) {
        if let Some(colors) = self.set_colors {
            copy.color_override = Some(colors);
        }
        for card_type in &self.added_card_types {
            if !copy.card_types.contains(card_type) {
                copy.card_types.push(*card_type);
            }
        }
        for subtype in &self.added_subtypes {
            if !copy.subtypes.contains(subtype) {
                copy.subtypes.push(*subtype);
            }
        }
        if let Some((power, toughness)) = self.set_base_power_toughness {
            copy.base_power = Some(crate::card::PtValue::Fixed(power));
            copy.base_toughness = Some(crate::card::PtValue::Fixed(toughness));
        }
    }
}

/// Propose the number of spell copies so replacements may change it. `None`
/// while a replacement-order choice is pending.
fn proposed_spell_copy_count(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    copier: crate::ids::PlayerId,
    copy_count: usize,
) -> Result<Option<usize>, ExecutionError> {
    use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
    use crate::events::{KeywordActionEvent, KeywordActionKind};
    if copy_count == 0
        || !crate::static_abilities::misc::event_amount_replacement::may_have_keyword_action_replacements(game)
    {
        return Ok(Some(copy_count));
    }
    let event = crate::events::Event::new_with_provenance(
        KeywordActionEvent::new(
            KeywordActionKind::CopySpell,
            copier,
            ctx.source,
            u32::try_from(copy_count).unwrap_or(u32::MAX),
        ),
        ctx.provenance,
    );
    match process_trait_event_with_execution_context(game, event, ctx)? {
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => Ok(Some(
            crate::events::downcast_event::<KeywordActionEvent>(event.inner())
                .filter(|action| action.action == KeywordActionKind::CopySpell)
                .map(|action| action.amount as usize)
                .unwrap_or(copy_count),
        )),
        TraitEventResult::Prevented => Ok(Some(0)),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if ctx.decision_maker.awaiting_choice() {
                Ok(None)
            } else {
                Err(ExecutionError::InternalError(
                    "copy proposal suspended without a decision".into(),
                ))
            }
        }
        TraitEventResult::Replaced { .. } | TraitEventResult::Expanded { .. } => Err(
            ExecutionError::InternalError("copy proposals only accept amount replacements".into()),
        ),
    }
}

impl EffectExecutor for CopySpellEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let copy_count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Resolve the stack object(s) to copy. `ChooseSpec::All` is a real
        // fanout instruction (for example, "copy all spells you control"),
        // while ordinary target/object specs still resolve to one object.
        // Snapshot the IDs before creating copies so a broad stack filter can
        // never recursively include the copies it just created.
        // A referenced activation remains on the stack independently of its
        // permanent. Tagged source snapshots may already refer to the new zone
        // object, so recover the event's exact activation identity here.
        let referenced_activation = ctx
            .triggering_event
            .as_ref()
            .and_then(|event| event.downcast::<crate::events::AbilityActivatedEvent>())
            .filter(|activation| {
                !self.target.is_target()
                    && matches!(self.target.base(), ChooseSpec::Tagged(tag)
                        if ctx.get_tagged_all(tag).is_some_and(|snapshots|
                            snapshots.iter().any(|snapshot|
                                snapshot.object_id == activation.source
                                    || activation.snapshot.as_ref().is_some_and(|source|
                                        source.stable_id == snapshot.stable_id))))
            });
        let source_spell_departed = matches!(self.target, ChooseSpec::Source)
            && departed_source_spell_lki(game, ctx, ctx.source).is_some();
        let target_ids = if let Some(activation) = referenced_activation {
            vec![activation.source]
        } else if source_spell_departed {
            vec![ctx.source]
        } else {
            match resolve_objects_for_effect(game, ctx, &self.target) {
                Ok(targets) => targets,
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
                Err(error) => return Err(error),
            }
        };
        if target_ids.is_empty() {
            return if self.target.count().min == 0 {
                Ok(EffectOutcome::with_objects(Vec::new()))
            } else {
                Err(ExecutionError::InvalidTarget)
            };
        }
        let copier = resolve_player_filter(game, &self.copier, ctx)?;
        // CR 707.10, 614.1a: "If you would copy a spell one or more times,
        // instead copy it that many times plus an additional time" changes
        // the number of copies of a spell (not of an ability).
        let Some(spell_copy_count) = proposed_spell_copy_count(game, ctx, copier, copy_count)?
        else {
            return Ok(EffectOutcome::count(0));
        };
        let mut additional_copies = Vec::new();
        let mut created_ids = Vec::with_capacity(copy_count.saturating_mul(target_ids.len()));
        let mut prevented_copy = false;

        for target_id in target_ids {
            if source_spell_departed
                && let Some((target, original_entry)) =
                    departed_source_spell_lki(game, ctx, target_id)
            {
                for _ in 0..copy_count {
                    let Some(copy_id) = create_stack_copy_from_object(
                        game,
                        &target,
                        target_id,
                        &original_entry,
                        copier,
                        &self.removed_supertypes,
                        |copy| self.apply_copy_characteristic_modifiers(copy),
                        None,
                    )?
                    else {
                        prevented_copy = true;
                        continue;
                    };
                    created_ids.push(copy_id);
                    queue_copy_becomes_targeted_events(game, ctx, copy_id);
                    game.queue_trigger_event(
                        ctx.provenance,
                        TriggerEvent::new_with_provenance(
                            SpellCopiedEvent::new(copy_id, copier),
                            ctx.provenance,
                        ),
                    );
                }
                continue;
            }
            let Some(original_entry) = stack_entry_for_copy_target(
                game,
                target_id,
                ctx,
                super::counter::counter_target_stack_kind(&self.target),
            )?
            else {
                continue;
            };
            let target = game
                .object(target_id)
                .cloned()
                .or_else(|| {
                    // An ability named by its own stack id copies from its source.
                    original_entry
                        .is_ability
                        .then(|| game.object(original_entry.object_id).cloned())
                        .flatten()
                })
                .or_else(|| {
                    original_entry
                        .is_ability
                        .then(|| {
                            original_entry.source_snapshot.as_ref().map(|snapshot| {
                                Object::token_copy_from_snapshot(
                                    snapshot,
                                    target_id,
                                    snapshot.owner,
                                )
                            })
                        })
                        .flatten()
                })
                .ok_or(ExecutionError::ObjectNotFound(target_id))?;
            let this_count = if original_entry.is_ability {
                copy_count
            } else {
                spell_copy_count
            };
            for index in 0..this_count {
                let Some(copy_id) = create_stack_copy_from_object(
                    game,
                    &target,
                    target_id,
                    &original_entry,
                    copier,
                    &self.removed_supertypes,
                    |copy| self.apply_copy_characteristic_modifiers(copy),
                    None,
                )?
                else {
                    prevented_copy = true;
                    continue;
                };
                created_ids.push(copy_id);
                if index >= copy_count {
                    additional_copies.push(copy_id);
                }

                queue_copy_becomes_targeted_events(game, ctx, copy_id);

                // Only copying a spell emits the spell-copied event. The same
                // effect type also represents activated/triggered ability
                // copies, which must not trigger magecraft-like abilities.
                if !original_entry.is_ability {
                    game.queue_trigger_event(
                        ctx.provenance,
                        TriggerEvent::new_with_provenance(
                            SpellCopiedEvent::new(copy_id, copier),
                            ctx.provenance,
                        ),
                    );
                }
            }
        }

        if created_ids.is_empty() {
            return Ok(if prevented_copy {
                EffectOutcome::protected()
            } else if copy_count == 0 {
                EffectOutcome::with_objects(Vec::new())
            } else {
                EffectOutcome::target_invalid()
            });
        }

        if !additional_copies.is_empty() {
            // "You may choose new targets for the additional copy" (CR
            // 707.10c): offered for each copy the replacement added.
            ctx.store_outcome(
                crate::effect::EffectId::ADDITIONAL_COPIES,
                EffectOutcome::with_objects(additional_copies),
            );
            crate::effects::ChooseNewTargetsEffect::new(
                crate::effect::EffectId::ADDITIONAL_COPIES,
                true,
            )
            .execute_child(game, ctx)?;
        }

        Ok(EffectOutcome::with_objects(created_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to copy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{Effect, Value};
    use crate::events::EventKind;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn copied_and_resolving_sources_preserve_exact_defender_bindings() {
        // Reconstructed source contract, UNRUN.
        let mut game=setup_game();let source=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Attacker").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build(),PlayerId(0),Zone::Battlefield);
        game.add_entering_attacker(source,crate::combat_state::AttackTarget::Player(PlayerId(1)));let attacking=game.retain_attacking_role(source,&crate::combat_state::AttackTarget::Player(PlayerId(1)));
        for reference in [attacking,crate::combat_state::DefendingPlayerReference::Selected(PlayerId(1)),crate::combat_state::DefendingPlayerReference::KnownAbsent,crate::combat_state::DefendingPlayerReference::Missing]{
            let object=game.object(source).unwrap().clone();let mut entry=StackEntry::ability(source,PlayerId(0),vec![Effect::gain_life(1)]);entry.defending_player_reference=Some(reference);
            let copy=create_stack_copy_from_object(&mut game,&object,source,&entry,PlayerId(0),&[],|_|{},None).unwrap().unwrap();assert_eq!(game.stack.iter().find(|entry|entry.object_id==copy).unwrap().defending_player_reference,Some(reference));
            let mut dm=crate::decision::SelectFirstDecisionMaker;let mut ctx=ExecutionContext::new(source,PlayerId(0),&mut dm);ctx.combat.defending_player_reference=Some(reference);assert_eq!(resolving_source_stack_entry(&ctx).defending_player_reference,Some(reference));
        }
    }

    fn create_instant_on_stack(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(1), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();

        let id = game.create_object_from_card(&card, controller, Zone::Stack);

        // Add stack entry using the constructor
        let entry = StackEntry::new(id, controller);
        game.stack.push(entry);

        id
    }

    #[test]
    fn copy_prohibition_is_checked_before_allocation_and_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell = create_instant_on_stack(&mut game, "Protected spell", alice);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::cant_be_copied(),
            )
            .in_zones(vec![Zone::Stack]),
        );
        let before = game.stack.len();
        let mut ctx = ExecutionContext::new_default(spell, alice);
        let result = CopySpellEffect::single(ChooseSpec::Source)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(result.status, crate::effect::OutcomeStatus::Protected);
        assert_eq!(game.stack.len(), before);
        assert!(game.take_pending_trigger_events().is_empty());
        // An ability of the same protected source can still be copied.
        let source = game.object(spell).unwrap().clone();
        let mut ability = StackEntry::new(spell, alice);
        ability.is_ability = true;
        let copy = create_stack_copy_from_object(
            &mut game,
            &source,
            spell,
            &ability,
            alice,
            &[],
            |_| {},
            None,
        )
        .unwrap()
        .expect("independent ability copies");
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.object_id == copy && entry.is_ability)
        );
    }

    #[test]
    fn departed_spell_copy_uses_the_captured_prohibition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell = create_instant_on_stack(&mut game, "Protected LKI", alice);
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::cant_be_copied(),
            )
            .in_zones(vec![Zone::Stack]),
        );
        let source = game.object(spell).unwrap().clone();
        let entry = game.stack.pop().unwrap();
        game.remove_object(spell);
        assert!(
            create_stack_copy_from_object(
                &mut game,
                &source,
                spell,
                &entry,
                alice,
                &[],
                |_| panic!("a prohibited copy must not be customized"),
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(game.stack.is_empty());
    }

    #[test]
    fn ordinary_copy_preserves_choices_but_a_later_prohibition_prevents_it() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell = create_instant_on_stack(&mut game, "Ordinary spell", alice);
        let mut entry = game.stack.last().unwrap().clone();
        entry.x_value = Some(3);
        entry.chosen_modes = Some(vec![0, 1]);
        let copy = create_stack_copy(&mut game, spell, &entry, alice, &[], None)
            .unwrap()
            .unwrap();
        let copied = game
            .stack
            .iter()
            .find(|entry| entry.object_id == copy)
            .unwrap();
        assert_eq!(copied.x_value, Some(3));
        assert_eq!(copied.chosen_modes, Some(vec![0, 1]));
        game.object_mut(spell).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::cant_be_copied(),
            ),
        );
        assert!(
            create_stack_copy(&mut game, spell, &entry, alice, &[], None)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn copied_player_targets_are_reported_once_per_distinct_participant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell = create_instant_on_stack(&mut game, "Repeated target spell", bob);
        game.stack.last_mut().unwrap().targets = vec![
            Target::Player(alice),
            Target::Player(alice),
            Target::Player(bob),
        ];
        let mut ctx = ExecutionContext::new_default(spell, bob);
        let outcome = CopySpellEffect::single(ChooseSpec::SpecificObject(spell))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let crate::effect::OutcomeValue::Objects(copies) = outcome.value else {
            panic!("copy");
        };
        let events = game.take_pending_trigger_events();
        let targeted: Vec<_> = events
            .iter()
            .filter_map(|event| event.downcast::<BecomesTargetedEvent>())
            .collect();
        assert_eq!(targeted.len(), 2);
        assert_eq!(targeted[0].target_player(), Some(alice));
        assert_eq!(targeted[1].target_player(), Some(bob));
        assert!(targeted.iter().all(|event| event.source == copies[0]
            && event.source_controller == bob
            && !event.by_ability));
    }

    #[test]
    fn test_copy_spell_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with the copy ID
        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let copy_id = ids[0];

            // Copy should be on stack
            let copy_obj = game.object(copy_id).unwrap();
            assert_eq!(copy_obj.zone, Zone::Stack);
            assert_eq!(copy_obj.name, "Lightning Bolt");
            assert_eq!(game.controller_of(copy_obj), alice);

            // Stack should have 2 entries (original + copy)
            assert_eq!(game.stack.len(), 2);

            // Copy should be on top (last in vec)
            assert_eq!(game.stack.last().unwrap().object_id, copy_id);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn spell_copy_characteristic_exceptions_become_copiable_values() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(2), "Winged Herald")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wizard])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);
        game.stack.push(StackEntry::new(spell_id, alice));

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![ResolvedTarget::Object(spell_id)];
        let effect = CopySpellEffect::single(ChooseSpec::spell())
            .with_added_subtypes(vec![Subtype::Spirit])
            .with_set_base_power_toughness(Some((1, 1)));
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(copy_ids) = outcome.value else {
            panic!("expected the copy object")
        };
        let copy = game
            .object(copy_ids[0])
            .expect("copy should be on stack")
            .clone();
        assert_eq!(copy.card_types, [CardType::Creature]);
        assert_eq!(copy.subtypes, [Subtype::Wizard, Subtype::Spirit]);
        assert_eq!(copy.base_power, Some(crate::card::PtValue::Fixed(1)));
        assert_eq!(copy.base_toughness, Some(crate::card::PtValue::Fixed(1)));

        let second_copy_id = game.new_object_id();
        let second_copy = Object::spell_copy_of(&copy, second_copy_id, alice);
        assert_eq!(second_copy.subtypes, [Subtype::Wizard, Subtype::Spirit]);
        assert_eq!(second_copy.base_power, Some(crate::card::PtValue::Fixed(1)));
        assert_eq!(
            second_copy.base_toughness,
            Some(crate::card::PtValue::Fixed(1))
        );
    }

    #[test]
    fn test_copy_spell_preserves_chosen_modes() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Modal Spell", alice);
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.chosen_modes = Some(vec![1, 3]);
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let copy_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("Expected Objects result"),
        };

        let copy_entry = game
            .stack
            .iter()
            .find(|e| e.object_id == copy_id)
            .expect("copy on stack");
        assert_eq!(copy_entry.chosen_modes, Some(vec![1, 3]));
    }

    #[test]
    fn test_copy_spell_preserves_announced_target_distribution() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let spell_id = create_instant_on_stack(&mut game, "Distributed Spell", alice);
        let spec = ChooseSpec::WithCount(
            Box::new(ChooseSpec::AnyTarget),
            crate::effect::ChoiceCount::exactly(2),
        );
        let entry = game
            .stack
            .iter_mut()
            .find(|entry| entry.object_id == spell_id)
            .expect("original stack entry");
        entry.targets = vec![Target::Player(alice), Target::Player(bob)];
        entry.target_assignments = vec![crate::game_state::TargetAssignment {
            spec: spec.clone(),
            range: 0..2,
        }];
        entry.target_distributions = vec![crate::game_state::TargetDistribution {
            spec,
            range: 0..2,
            allocations: vec![(Target::Player(alice), 1), (Target::Player(bob), 2)],
        }];

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];
        let result = CopySpellEffect::single(ChooseSpec::spell())
            .execute(&mut game, &mut ctx)
            .expect("copy distributed spell");
        let crate::effect::OutcomeValue::Objects(copies) = result.value else {
            panic!("expected copied spell object");
        };
        let copy = game
            .stack
            .iter()
            .find(|entry| entry.object_id == copies[0])
            .expect("copy stack entry");
        assert_eq!(
            copy.target_distributions[0].allocations,
            vec![(Target::Player(alice), 1), (Target::Player(bob), 2)]
        );
    }

    #[test]
    fn test_copy_spell_preserves_spliced_text_and_provenance_until_stack_exit() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let spell_id = create_instant_on_stack(&mut game, "Spliced Spell", alice);
        let splice_card = crate::ids::StableId::from_raw(991);

        let spell = game.object_mut(spell_id).expect("original spell exists");
        spell.spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::gain_life(1),
            ])
            .into(),
        );
        assert!(spell.begin_splice_cast_overlay());
        let mut active_program = spell.spell_effect_owned().expect("base program");
        active_program.extend(crate::resolution::ResolutionProgram::from_effects(vec![
            crate::effect::Effect::draw(1),
        ]));
        spell.spell_effect = Some(active_program.into());
        game.stack
            .iter_mut()
            .find(|entry| entry.object_id == spell_id)
            .expect("original stack entry")
            .spliced_cards = vec![splice_card];

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];
        let result = CopySpellEffect::single(ChooseSpec::spell())
            .execute(&mut game, &mut ctx)
            .expect("copy spliced spell");
        let crate::effect::OutcomeValue::Objects(copies) = result.value else {
            panic!("expected copied spell object");
        };
        let copy_id = copies[0];
        assert_eq!(
            game.stack
                .iter()
                .find(|entry| entry.object_id == copy_id)
                .expect("copy stack entry")
                .spliced_cards,
            vec![splice_card]
        );
        assert_eq!(
            game.object(copy_id)
                .and_then(|copy| copy.spell_effect.as_ref())
                .expect("copy retains active spliced program")
                .flattened_default_effects()
                .len(),
            2
        );

        game.stack.retain(|entry| entry.object_id != copy_id);
        let graveyard_id = game
            .move_object_by_effect(copy_id, Zone::Graveyard)
            .expect("copy can leave stack for overlay regression");
        assert_eq!(
            game.object(graveyard_id)
                .and_then(|copy| copy.spell_effect.as_ref())
                .expect("pre-splice program restored")
                .flattened_default_effects()
                .len(),
            1
        );
    }

    #[test]
    fn test_copy_spell_multiple() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Shock", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::new(ChooseSpec::spell(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 3);

            // Stack should have 4 entries (original + 3 copies)
            assert_eq!(game.stack.len(), 4);

            // All copies should be on stack
            for copy_id in ids {
                let copy_obj = game.object(copy_id).unwrap();
                assert_eq!(copy_obj.zone, Zone::Stack);
                assert_eq!(copy_obj.name, "Shock");
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn gravestorm_count_copies_once_per_battlefield_to_graveyard_move() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let spell_id = create_instant_on_stack(&mut game, "Gravestorm Probe", alice);
        game.stack.clear();

        let permanent = CardBuilder::new(CardId::from_raw(2), "Returning Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let first_incarnation = game.create_object_from_card(&permanent, alice, Zone::Battlefield);
        let graveyard_incarnation = game
            .move_object_by_effect(first_incarnation, Zone::Graveyard)
            .expect("permanent should enter the graveyard");
        let second_incarnation = game
            .move_object_by_effect(graveyard_incarnation, Zone::Battlefield)
            .expect("permanent should return");
        game.move_object_by_effect(second_incarnation, Zone::Graveyard)
            .expect("the returned permanent should enter the graveyard again");

        let exiled = CardBuilder::new(CardId::from_raw(3), "Exiled Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let exiled_id = game.create_object_from_card(&exiled, alice, Zone::Battlefield);
        game.move_object_by_effect(exiled_id, Zone::Exile)
            .expect("battlefield-to-exile must not count");

        let discarded = CardBuilder::new(CardId::from_raw(4), "Discarded Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let discarded_id = game.create_object_from_card(&discarded, alice, Zone::Hand);
        game.move_object_by_effect(discarded_id, Zone::Graveyard)
            .expect("hand-to-graveyard must not count");

        let mut ctx = ExecutionContext::new_default(spell_id, alice);
        let effect = CopySpellEffect::new(
            ChooseSpec::Source,
            Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::died(
                crate::target::ObjectFilter::default(),
            )),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(copies) = result.value else {
            panic!("expected Gravestorm copies, got {result:#?}");
        };
        assert_eq!(
            copies.len(),
            2,
            "each qualifying move is counted separately"
        );
        assert_eq!(game.stack.len(), 2);
    }

    #[test]
    fn test_copy_all_matching_spells_fans_out_once_per_original() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let first = create_instant_on_stack(&mut game, "First Spell", alice);
        let second = create_instant_on_stack(&mut game, "Second Spell", alice);
        let originals = [first, second];
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CopySpellEffect::single(ChooseSpec::All(
            crate::filter::ObjectFilter::spell().controlled_by(crate::target::PlayerFilter::You),
        ));

        let result = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(copies) = result.value else {
            panic!("expected copied stack objects, got {result:#?}");
        };
        assert_eq!(copies.len(), originals.len());
        assert_eq!(game.stack.len(), originals.len() + copies.len());
        for copy in copies {
            assert!(!originals.contains(&copy));
            assert_eq!(game.object(copy).expect("copy exists").zone, Zone::Stack);
        }
    }

    #[test]
    fn test_copy_spell_queues_spell_copied_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Shock", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let _ = effect.execute(&mut game, &mut ctx).unwrap();

        let events = game.take_pending_trigger_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), EventKind::SpellCopied);
    }

    #[test]
    fn test_copy_spell_preserves_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        // Set original spell's targets (using game_state::Target)
        let target = crate::game_state::Target::Player(bob);
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.targets.push(target);
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let copy_id = ids[0];

            // Copy's stack entry should have same targets
            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.targets.len(), 1);
            assert!(matches!(
                copy_entry.targets[0],
                crate::game_state::Target::Player(p) if p == bob
            ));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_preserves_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let spell_id = create_instant_on_stack(&mut game, "Fireball", alice);

        // Set X value on original
        if let Some(entry) = game.stack.iter_mut().find(|e| e.object_id == spell_id) {
            entry.x_value = Some(5);
        }

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let copy_id = ids[0];

            // Copy should preserve X value
            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.x_value, Some(5));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_can_copy_currently_resolving_source_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let spell_id = create_instant_on_stack(&mut game, "Resolving Bolt", alice);
        game.stack.clear();

        let mut ctx = ExecutionContext::new_default(spell_id, alice);

        let effect = CopySpellEffect::single(ChooseSpec::Source);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let copy_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("Expected Objects result"),
        };

        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].object_id, copy_id);
        assert_eq!(
            game.object(copy_id).expect("copy exists").name,
            "Resolving Bolt"
        );
    }

    #[test]
    fn test_copy_spell_not_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // Create a spell NOT on stack
        let card = CardBuilder::new(CardId::from_raw(1), "Lightning Bolt")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_copy_spell_different_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        // Alice's spell on stack
        let spell_id = create_instant_on_stack(&mut game, "Lightning Bolt", alice);

        // Bob copies it
        let mut ctx = ExecutionContext::new_default(source, bob);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(spell_id)];

        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let copy_id = ids[0];

            // Copy should be controlled by Bob
            let copy_obj = game.object(copy_id).unwrap();
            assert_eq!(game.controller_of(copy_obj), bob);

            let copy_entry = game.stack.iter().find(|e| e.object_id == copy_id).unwrap();
            assert_eq!(copy_entry.controller, bob);
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_copy_spell_clone_box() {
        let effect = CopySpellEffect::single(ChooseSpec::spell());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CopySpellEffect"));
    }
}

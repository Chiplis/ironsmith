//! Counter spell effect implementation.

use crate::ability::AbilityKind;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::zones::apply_zone_change_with_context_and_additional_effects_with_outputs;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::CounterEffect;

/// The stack-object kind a stack-targeting effect's target is restricted to.
pub(crate) fn counter_target_stack_kind(
    spec: &ChooseSpec,
) -> Option<crate::filter::StackObjectKind> {
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter.stack_kind,
        _ => None,
    }
}

fn counter_one_stack_object_of_kind_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target_id: ObjectId,
    kind: Option<crate::filter::StackObjectKind>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let mut receipts = Vec::new();
            let mut published_outputs = Vec::new();
            let original = counter_one_stack_object_of_kind_inner(
                game,
                ctx,
                target_id,
                kind,
                None,
                &mut receipts,
                &mut published_outputs,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let mut original = crate::effects::CompletedEffectOutputs::aggregate_only(original);
            original.retain_published_references(published_outputs);
            crate::effects::zones::finish_zone_change_receipts_with_outputs(
                game, ctx, original, receipts,
            )
        },
    )
}

fn counter_one_stack_object_of_kind_inner(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target_id: ObjectId,
    kind: Option<crate::filter::StackObjectKind>,
    exile_permission: Option<ironsmith_core::CounterExilePermission>,
    receipts: &mut Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
    published_outputs: &mut Vec<crate::effects::PublishedEffectOutputs>,
) -> Result<EffectOutcome, ExecutionError> {
    use crate::filter::StackObjectKind;

    if exile_permission.is_some() && (!game.object(target_id)
        .is_some_and(|object| object.zone == Zone::Stack)
        || !game.stack.iter().any(|entry| entry.object_id == target_id && !entry.is_ability))
    {
        return Ok(EffectOutcome::target_invalid());
    }
    let kind = if exile_permission.is_some() {
        Some(crate::filter::StackObjectKind::Spell)
    } else { kind };
    // An ability named by its own stack id is exactly that stack object.
    if let Some(index) = game
        .stack
        .iter()
        .position(|entry| entry.ability_id == Some(target_id))
    {
        return Ok(commit_countered_ability(game, index));
    }

    if !game.can_be_countered(target_id) {
        return Ok(EffectOutcome::protected());
    }

    // Abilities use their source's object ID but are independent stack entries
    // (a storm trigger shares its spell's ID). Counter the entry of the kind
    // the effect targets: "target activated or triggered ability" never
    // counters the spell below it, and prefers the most recent ability. A
    // source's spell protection and zone-change replacements do not apply
    // when removing an ability from the stack.
    let ability_index = match kind {
        Some(StackObjectKind::Spell) => None,
        Some(
            kind @ (StackObjectKind::Ability
            | StackObjectKind::ActivatedAbility
            | StackObjectKind::TriggeredAbility),
        ) => game.stack.iter().rposition(|entry| {
            entry.object_id == target_id
                && <crate::filter::ObjectFilter as crate::filter::ObjectFilterExt>::stack_entry_matches_kind(entry, kind)
        }),
        _ => game
            .stack
            .iter()
            .position(|entry| entry.object_id == target_id)
            .filter(|&index| game.stack[index].is_ability),
    };
    if let Some(index) = ability_index {
        return Ok(commit_countered_ability(game, index));
    }
    if matches!(
        kind,
        Some(
            StackObjectKind::Ability
                | StackObjectKind::ActivatedAbility
                | StackObjectKind::TriggeredAbility
        )
    ) {
        return Ok(EffectOutcome::target_invalid());
    }

    // Check if the spell can't be countered
    if let Some(obj) = game.object(target_id) {
        let abilities = game
            .current_abilities(target_id)
            .unwrap_or_else(|| obj.abilities_vec());
        let cant_be_countered = abilities.iter().any(|ability| {
            if let AbilityKind::Static(s) = &ability.kind {
                // Only a self-protection capability applies on this spell.
                // Battlefield-wide restrictions are evaluated by the tracker;
                // their display text does not protect their source on the stack.
                s.cant_be_countered()
                    || s.id() == crate::static_abilities::StaticAbilityId::CantBeCountered
            } else {
                false
            }
        });
        if cant_be_countered {
            // Spell can't be countered - effect does nothing
            return Ok(EffectOutcome::protected());
        }
    }

    // Find the stack entry for this object
    Ok(
        if game
            .stack
            .iter()
            .any(|e| e.object_id == target_id && !e.is_ability)
        {
            // Capture identity before the countered spell changes zones.
            let countered_info = game.object(target_id).map(|obj| {
                (
                    obj.stable_id,
                    game.current_controller(target_id).unwrap_or(obj.owner),
                )
            });
            let countered_snapshot = game.object(target_id).map(|object| {
                crate::snapshot::ObjectSnapshot::try_from_object_with_calculated_characteristics(
                    object, game,
                )
            }).transpose()?;
            let lookback_source_snapshots = game.trigger_source_lookback_snapshots();
            let eligible_permission = exile_permission.filter(|permission| {
                match permission.gate {
                    ironsmith_core::CounterExileGate::AnySpell => true,
                    ironsmith_core::CounterExileGate::PermanentSpell => countered_snapshot
                        .as_ref().is_some_and(|snapshot| snapshot.card_types.iter().any(|kind|
                            matches!(kind, crate::types::CardType::Artifact
                                | crate::types::CardType::Battle | crate::types::CardType::Creature
                                | crate::types::CardType::Enchantment | crate::types::CardType::Planeswalker))),
                }
            });
            let mut additional_effects = ctx.additional_replacement_effects_snapshot();
            if eligible_permission.is_some() {
                // This replacement exists only during this one counter event.
                // A nonpermanent remains a legal target and goes to its normal
                // destination. No global or later-movement replacement survives.
                additional_effects.push(crate::replacement::ReplacementEffect::with_matcher(
                    ctx.source, ctx.controller,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(target_id),
                        Some(Zone::Stack), Some(Zone::Graveyard),
                    ),
                    crate::replacement::ReplacementAction::ChangeDestination(Zone::Exile),
                ).with_priority_override(crate::events::ReplacementPriority::SelfReplacement));
            }
            let committed = apply_zone_change_with_context_and_additional_effects_with_outputs(
                game,
                target_id,
                Zone::Stack,
                Zone::Graveyard,
                ctx.cause.clone(),
                ctx,
                &additional_effects,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            crate::effects::PublishedEffectOutputs::append_distinct(
                published_outputs,
                committed.published_outputs,
            );
            let receipt = committed.receipt;
            let outcome = receipt.original.clone();
            receipts.push((target_id, receipt));
            let mut countered_spell = false;
            match outcome {
                EventOutcome::Prevented => return Ok(EffectOutcome::prevented()),
                EventOutcome::Proceed(change) => {
                    // The common commit has already removed the departing spell
                    // from its zone index, preserving independent abilities that
                    // share its source id. Do not run entry replacements again.
                    countered_spell = change.new_object_id.is_some();
                    if change.final_zone == Zone::Exile {
                        if let (Some(permission), Some(exiled)) =
                            (eligible_permission, change.new_object_id)
                        {
                            grant_countered_exile_permission(game, ctx, exiled, permission);
                        }
                        for new_id in change.new_object_ids {
                            game.add_exiled_with_source_link(ctx.source, new_id);
                        }
                    }
                }
                EventOutcome::Replaced => {
                    if let Some(idx) = game
                        .stack
                        .iter()
                        .position(|e| e.object_id == target_id && !e.is_ability)
                    {
                        let entry = game.stack.remove(idx);
                        countered_spell = !entry.is_ability;
                    }
                }
                EventOutcome::NotApplicable => return Ok(EffectOutcome::target_invalid()),
            }

            if !game
                .stack
                .iter()
                .any(|e| e.object_id == target_id && !e.is_ability)
            {
                if let Some((stable_id, controller)) = countered_info {
                    game.record_ui_effect_event(
                        "spell_countered",
                        Some(controller),
                        None,
                        vec![stable_id],
                        None,
                        None,
                    );
                    if countered_spell {
                        let mut event = crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::SpellCounteredEvent::new(
                                target_id,
                                controller,
                                countered_snapshot,
                            )
                            .with_cause(ctx.cause.clone())
                            .with_complete_source_lookback(),
                            ctx.provenance,
                        )
                        .with_lookback_source_snapshots(lookback_source_snapshots);
                        if game.object(ctx.source).is_none()
                            && let Some(snapshot) = ctx.source_snapshot.clone()
                        {
                            event = event.with_source_snapshot(snapshot);
                        }
                        return Ok(EffectOutcome::resolved().with_event(event));
                    }
                }
                EffectOutcome::resolved()
            } else {
                EffectOutcome::target_invalid()
            }
        } else {
            // Target is no longer on the stack
            EffectOutcome::target_invalid()
        },
    )
}

/// Consume only the committed original counter arrival, before receipt
/// additions run. Replay/choice suspension is owned by the surrounding counter
/// transaction, so no permission survives an uncommitted move. Departure during
/// receipt additions invalidates this exact ID; there is no stable-card rebinding.
fn grant_countered_exile_permission(
    game: &mut GameState,
    ctx: &ExecutionContext,
    exiled: ObjectId,
    permission: ironsmith_core::CounterExilePermission,
) {
    if !game.object(exiled).is_some_and(|object| object.zone == Zone::Exile) {
        return;
    }
    use crate::grant::Grantable;
    use crate::grant_registry::{GrantSource, PlayFromConstraints};
    let source = GrantSource::Effect {
        source_id: ctx.source,
        expires_end_of_turn: u32::MAX,
    };
    let registry = &mut game.effect_store.grant_registry;
    registry.grant_to_card(exiled, Zone::Exile, ctx.controller,
        Grantable::AlternativeCast(
            crate::alternative_cast::AlternativeCastingMethod::cast_from_zone_with_total_cost(
                "Countered spell casting price", Zone::Exile,
                crate::cost::TotalCost::from_costs(Vec::new()), None, false,
            ),
        ), source.clone());
    let mut spells = crate::target::ObjectFilter::default();
    spells.zone = None;
    spells.excluded_card_types.push(crate::types::CardType::Land);
    registry.grants.last_mut().expect("inserted exact counter grant").filter = Some(spells);
    if permission.allow_land {
        registry.grant_play_from_to_card(exiled, Zone::Exile, ctx.controller,
            PlayFromConstraints::default(), source);
        let mut lands = crate::target::ObjectFilter::land();
        lands.zone = None;
        registry.grants.last_mut().expect("inserted exact land grant").filter = Some(lands);
    }
}

/// Counter the stack object at `index` (CR 701.6a).
///
/// Abilities share their source's object ID, so a caller that already knows
/// which stack entry it means (for example, the spell or ability a ward
/// trigger is countering) removes that exact entry rather than the first one
/// with a matching ID.
pub(crate) fn counter_stack_entry_at(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    index: usize,
) -> Result<EffectOutcome, ExecutionError> {
    counter_stack_entry_at_with_outputs(game, ctx, index)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn counter_stack_entry_at_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    index: usize,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let Some(entry) = game.stack.get(index) else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::target_invalid(),
                ));
            };
            let object_id = entry.object_id;
            if !entry.is_ability {
                return counter_one_stack_object_of_kind_with_outputs(
                    game,
                    ctx,
                    object_id,
                    Some(crate::filter::StackObjectKind::Spell),
                );
            }
            Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                commit_countered_ability(game, index),
            ))
        },
    )
}

/// One physical removal owner for exact and source-relative ability targets.
/// The caller selects the entry before commitment; spell protection and zone
/// replacements never participate in this ability removal.
fn commit_countered_ability(game: &mut GameState, index: usize) -> EffectOutcome {
    if !game.stack.get(index).is_some_and(|entry| entry.is_ability) {
        return EffectOutcome::target_invalid();
    }
    let entry = game.stack.remove(index);
    let outcome = countered_ability_outcome(game, &entry);
    super::copy_spell::discard_departed_ability_copy_object(game, &entry);
    outcome
}

/// Effect that counters a target spell on the stack.
///
/// This removes the spell from the stack and puts it into its owner's graveyard.
/// Abilities that are countered simply disappear.
///
/// # Fields
///
/// * `target` - Which spell to counter
///
/// # Example
///
/// ```ignore
/// // Counter target spell
/// let effect = CounterEffect::new(ChooseSpec::spell());
///
/// // Counter target creature spell
/// let effect = CounterEffect::new(ChooseSpec::creature_spell());
/// ```
fn countered_ability_outcome(
    game: &GameState,
    entry: &crate::game_state::StackEntry,
) -> EffectOutcome {
    let snapshot = entry
        .source_snapshot
        .clone()
        .or_else(|| crate::snapshot::ObjectSnapshot::from_object_id(game, entry.object_id));
    let objects = snapshot
        .map(|mut snapshot| {
            snapshot.object_id = entry.target_id();
            snapshot.zone = Zone::Stack;
            snapshot.controller = entry.controller;
            snapshot.stack_kind = Some(if entry.triggering_event.is_some() {
                crate::filter::StackObjectKind::TriggeredAbility
            } else {
                crate::filter::StackObjectKind::ActivatedAbility
            });
            snapshot
        })
        .into_iter()
        .collect::<Vec<_>>();
    EffectOutcome::resolved()
        .with_action_objects(
            crate::effect::PriorEffectAction::Countered,
            Some(entry.controller),
            objects.clone(),
        )
        .with_affected_object_memory(objects)
}

impl EffectExecutor for CounterEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Countered)
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                if !self.exile_permission_target_is_supported() {
                    return Err(ExecutionError::IncompleteEvidence(
                        "counter exile permission requires one explicit stack spell".into()));
                }
                let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                if target_ids.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                let kind = counter_target_stack_kind(&self.target);
                if self.exile_permission.is_some() && (target_ids.len() != 1
                    || matches!(kind, Some(crate::filter::StackObjectKind::Ability
                        | crate::filter::StackObjectKind::ActivatedAbility
                        | crate::filter::StackObjectKind::TriggeredAbility)))
                {
                    return Err(ExecutionError::IncompleteEvidence(
                        "counter exile permission requires one spell target".into()));
                }
                let mut outcomes = Vec::new();
                let mut receipts = Vec::new();
                let mut published_outputs = Vec::new();
                for target_id in target_ids {
                    outcomes.push(counter_one_stack_object_of_kind_inner(
                        game,
                        ctx,
                        target_id,
                        kind,
                        self.exile_permission,
                        &mut receipts,
                        &mut published_outputs,
                    )?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                }
                let mut original = crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::aggregate(outcomes),
                );
                original.retain_published_references(published_outputs);
                crate::effects::zones::finish_zone_change_receipts_with_outputs(
                    game, ctx, original, receipts,
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to counter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{Effect, OutcomeStatus};
    use crate::effects::execute_effect;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{Object, ObjectKind};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;

    #[test]
    fn countered_spell_entry_error_restores_stack_and_entry_replacements() {
        for change_controller in [false, true] {
        for fails in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let creature = CardBuilder::new(CardId::new(), "Counter destination fixture")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
            let target = game.create_object_from_card(&creature, bob, Zone::Stack);
            let stable_id = game.object(target).unwrap().stable_id;
            game.stack.push(StackEntry::new(target, bob));
            let source = create_instant(&mut game, alice, Zone::Stack, "Counter source");
            let zone_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source, alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Graveyard),
                    ),
                    crate::replacement::ReplacementAction::ChangeDestination(Zone::Battlefield),
                ),
            );
            let controller_shot = change_controller.then(|| {
                game.effect_store.replacement_effects.add_one_shot_effect(
                    crate::replacement::ReplacementEffect::with_matcher(target, alice,
                        crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                        crate::replacement::ReplacementAction::EnterUnderControl(alice)),
                )
            });
            let mut program = vec![Effect::gain_life(2)];
            if fails {
                program.push(Effect::lose_life(crate::effect::Value::X));
            }
            let entry_shot = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    target, alice,
                    crate::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    crate::replacement::ReplacementAction::AsEntersProgram(
                        crate::resolution::ResolutionProgram::from_effects(program),
                    ),
                ),
            );
            game.take_pending_trigger_events();
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let result = execute_effect(
                &mut game,
                &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target))),
                &mut ctx,
            );
            assert!(!ctx.decision_maker.awaiting_choice());
            if fails {
                assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
                assert!(game.stack.iter().any(|entry| entry.object_id == target));
                assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert!(game.effect_store.replacement_effects.get_effect(zone_shot).is_some());
                assert!(game.effect_store.replacement_effects.get_effect(entry_shot).is_some());
                if let Some(id) = controller_shot { assert!(game.effect_store.replacement_effects.get_effect(id).is_some()); }
                assert!(game.take_pending_trigger_events().is_empty());
            } else {
                let outcome = result.unwrap();
                assert_eq!(outcome.status, OutcomeStatus::Succeeded);
                assert!(!game.stack.iter().any(|entry| entry.object_id == target));
                let entered = game.find_object_by_stable_id(stable_id).unwrap();
                assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
                assert_eq!(game.current_controller(entered), Some(if change_controller { alice } else { bob }));
                if let Some(id) = controller_shot { assert!(game.effect_store.replacement_effects.get_effect(id).is_none()); }
                let countered = outcome.events.iter().filter_map(|event| event.downcast::<crate::events::SpellCounteredEvent>()).collect::<Vec<_>>();
                assert_eq!(countered.len(), 1);
                assert_eq!(countered[0].spell, target);
                assert_eq!(countered[0].controller, bob);
                assert_eq!(game.player(alice).unwrap().life, 22);
                assert!(game.effect_store.replacement_effects.get_effect(zone_shot).is_none());
                assert!(game.effect_store.replacement_effects.get_effect(entry_shot).is_none());
                let events = game.take_pending_trigger_events();
                let gains = events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).collect::<Vec<_>>();
                assert_eq!(gains.len(), 1);
                assert_eq!(gains[0].amount, 2);
            }
        }
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_instant(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn counter_then_destroy_tagged_ability_source_works_with_resolving_source_gone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let artifact = CardBuilder::new(CardId::new(), "Artifact Ability Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_source = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        let artifact_stable = game.object(artifact_source).unwrap().stable_id;
        game.push_to_stack(StackEntry::ability(
            artifact_source,
            bob,
            vec![Effect::draw(1)],
        ));

        let sacrificed_source = CardBuilder::new(CardId::new(), "Sacrificed Source")
            .card_types(vec![CardType::Creature])
            .build();
        let resolving_source =
            game.create_object_from_card(&sacrificed_source, alice, Zone::Graveyard);

        let mut counter_filter = ObjectFilter::default().in_zone(Zone::Stack);
        counter_filter.stack_kind = Some(crate::filter::StackObjectKind::ActivatedAbility);
        counter_filter.card_types = vec![CardType::Artifact];
        let counter_tag = crate::tag::TagKey::from("countered_0");
        let counter = Effect::new(CounterEffect::new(ChooseSpec::target(ChooseSpec::Object(
            counter_filter,
        ))))
        .tag(counter_tag.clone());
        let destroy_filter = ObjectFilter::artifact()
            .in_zone(Zone::Battlefield)
            .match_tagged(
                counter_tag,
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            );
        let sequence = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            counter,
            Effect::destroy(ChooseSpec::Object(destroy_filter)),
        ]));

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(resolving_source, alice, &mut dm).with_targets(vec![
            crate::effects::ResolvedTarget::Object(artifact_source),
        ]);
        execute_effect(&mut game, &sequence, &mut ctx)
            .expect("counter/destroy sequence should resolve after its source was sacrificed");

        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == artifact_source),
            "the activated ability should be countered"
        );
        let artifact_after = game
            .find_object_by_stable_id(artifact_stable)
            .expect("artifact source remains tracked");
        assert_eq!(
            game.object(artifact_after)
                .expect("artifact after sequence")
                .zone,
            Zone::Graveyard,
            "the tagged source of the countered ability should be destroyed"
        );
        assert_eq!(
            game.object(resolving_source)
                .expect("sacrificed resolving source")
                .zone,
            Zone::Graveyard,
        );
    }

    #[test]
    fn counter_spell_honors_registered_stack_to_graveyard_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Target Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.stack.push(StackEntry::new(target_spell, bob));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let register = Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(target_spell),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            crate::effects::ReplacementApplyMode::OneShot,
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        execute_effect(&mut game, &register, &mut ctx)
            .expect("replacement registration should succeed");

        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");
        assert!(
            outcome.status.is_success(),
            "counter should resolve successfully"
        );
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == target_spell),
            "countered spell should be removed from the stack"
        );

        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be findable after the zone change");
        assert_eq!(
            game.object(moved_id)
                .expect("countered spell should still exist after being moved")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn counter_spell_can_replace_destination_and_battlefield_controller_in_one_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = CardBuilder::new(CardId::new(), "Creature Spell")
            .card_types(vec![CardType::Creature])
            .build();
        let target_spell = game.create_object_from_card(&creature, bob, Zone::Stack);
        let stable_id = game.object(target_spell).unwrap().stable_id;
        game.stack.push(StackEntry::new(target_spell, bob));
        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);

        for register in [
            Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
                ChooseSpec::SpecificObject(target_spell),
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Battlefield,
                crate::effects::ReplacementApplyMode::OneShot,
            )),
            Effect::new(
                crate::effects::RegisterEnterUnderControlReplacementEffect::new(
                    ObjectFilter::specific(target_spell),
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn,
                ),
            ),
        ] {
            execute_effect(&mut game, &register, &mut ctx).unwrap();
        }
        execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve through both replacements");

        let moved = game.find_object_by_stable_id(stable_id).unwrap();
        let object = game.object(moved).unwrap();
        assert_eq!(object.zone, Zone::Battlefield);
        assert_eq!(game.controller_of(object), alice);
        assert!(
            !game.player(bob).unwrap().graveyard.contains(&moved),
            "the card must never visit its owner's graveyard"
        );
    }

    #[test]
    fn counter_spell_honors_own_from_anywhere_exile_replacement_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Self Replacing Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.object_mut(target_spell)
            .expect("target spell should exist")
            .abilities_mut()
            .push(
                crate::ability::Ability::static_ability(
                    StaticAbility::exile_to_exile_instead_of_graveyard(
                        ObjectFilter::source(),
                        PlayerFilter::Any,
                    ),
                )
                .in_zones(vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Graveyard,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Exile,
                    Zone::Command,
                ]),
            );
        game.stack.push(StackEntry::new(target_spell, bob));
        game.update_replacement_effects();

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert!(
            outcome.status.is_success(),
            "counter should resolve successfully"
        );
        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be tracked after moving");
        assert_eq!(
            game.object(moved_id)
                .expect("countered spell should still exist after moving")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn counter_spell_moves_spell_to_owners_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Target Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.stack.push(StackEntry::new(target_spell, bob));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == target_spell),
            "countered spell should leave the stack"
        );

        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be tracked after moving");
        let moved_obj = game
            .object(moved_id)
            .expect("countered spell should still exist after moving");
        assert_eq!(moved_obj.zone, Zone::Graveyard);
        assert_eq!(moved_obj.owner, bob);
    }

    #[test]
    fn countered_spell_copy_ceases_to_exist_at_the_next_sba_check() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let original = create_instant(&mut game, bob, Zone::Stack, "Original Spell");
        game.stack.push(StackEntry::new(original, bob));
        let original_object = game
            .object(original)
            .expect("original spell should exist")
            .clone();
        let copy_id = game.new_object_id();
        let copy = Object::spell_copy_of(&original_object, copy_id, bob);
        let copy_stable_id = copy.stable_id;
        game.add_object(copy);
        game.stack.push(StackEntry::new(copy_id, bob));
        let surviving_copy_id = game.new_object_id();
        game.add_object(Object::spell_copy_of(
            &original_object,
            surviving_copy_id,
            bob,
        ));
        game.stack.push(StackEntry::new(surviving_copy_id, bob));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(copy_id))),
            &mut ctx,
        )
        .expect("counter should resolve");

        let moved_copy = game
            .find_object_by_stable_id(copy_stable_id)
            .expect("the copy should move before state-based actions");
        assert!(game.object(moved_copy).is_some_and(|object| {
            object.kind == ObjectKind::SpellCopy && object.zone == Zone::Graveyard
        }));

        assert!(crate::rules::state_based::apply_state_based_actions(
            &mut game
        ).expect("replacement operation must finish without execution error"));
        assert!(
            game.find_object_by_stable_id(copy_stable_id).is_none(),
            "a countered spell copy must cease to exist rather than remain in a graveyard"
        );
        assert!(
            game.object(surviving_copy_id)
                .is_some_and(|object| object.zone == Zone::Stack),
            "a spell copy still on the stack must not be removed by the SBA"
        );
        assert!(game.object(original).is_some());
    }

    #[test]
    fn counter_ability_only_removes_it_from_the_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Ability Source")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        game.stack
            .push(StackEntry::ability(source, bob, vec![Effect::draw(1)]));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(source))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game.stack.iter().any(|entry| entry.object_id == source),
            "countered ability should disappear from the stack"
        );
        assert_eq!(
            game.object(source)
                .expect("ability source permanent should still exist")
                .zone,
            Zone::Battlefield
        );
    }

    #[test]
    fn kadenas_silencer_counter_all_opponent_abilities_leaves_your_stack_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let opponent_first_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Opponent Ability Source A")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let opponent_second_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Opponent Ability Source B")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let your_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Your Ability Source")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let opponent_spell = create_instant(&mut game, bob, Zone::Stack, "Opponent Spell");

        game.stack.push(StackEntry::ability(
            opponent_first_ability_source,
            bob,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::ability(
            opponent_second_ability_source,
            bob,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::ability(
            your_ability_source,
            alice,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::new(opponent_spell, bob));

        let silencer = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Kadena's Silencer")
                .card_types(vec![CardType::Creature])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(silencer, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::All(
                ObjectFilter::ability().controlled_by(PlayerFilter::Opponent),
            ))),
            &mut ctx,
        )
        .expect("Kadena's Silencer counter-all-abilities effect should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == opponent_first_ability_source),
            "first opponent ability should be countered"
        );
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == opponent_second_ability_source),
            "second opponent ability should be countered"
        );
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.object_id == your_ability_source && entry.is_ability),
            "your ability should not be countered"
        );
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.object_id == opponent_spell && !entry.is_ability),
            "opponent spell should not be countered by an ability-only filter"
        );
    }

    #[test]
    fn countering_a_spell_does_not_refund_paid_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Paid Spell")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
                .build(),
            bob,
            Zone::Hand,
        );
        game.player_mut(bob)
            .expect("bob exists")
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        assert!(
            game.try_pay_mana_cost_with_reason(
                bob,
                Some(target_spell),
                &ManaCost::from_symbols(vec![ManaSymbol::Blue]),
                0,
                crate::costs::PaymentReason::CastSpell,
            ).expect("checked fixture mana payment"),
            "bob should be able to pay for the spell before it is countered"
        );
        let stack_spell = game
            .move_object_by_effect(target_spell, Zone::Stack)
            .expect("paid spell should move to stack");
        game.stack.push(StackEntry::new(stack_spell, bob));
        assert_eq!(
            game.player(bob).expect("bob exists").mana_pool.total(),
            0,
            "mana should already be spent before the counter resolves"
        );

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(stack_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert_eq!(
            game.player(bob).expect("bob exists").mana_pool.total(),
            0,
            "countering a spell must not refund the mana already paid to cast it"
        );
    }
}

#[cfg(test)]
#[path = "counter_exile_permission_tests.rs"]
mod exile_permission_tests;

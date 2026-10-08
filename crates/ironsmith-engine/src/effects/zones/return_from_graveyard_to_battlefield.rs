//! Return from graveyard to battlefield effect implementation.

use super::battlefield_entry::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, resolve_battlefield_entry_counters,
};
use crate::continuous::Modification;
use crate::decisions::make_decision;
use crate::decisions::specs::objects::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
pub use ironsmith_core::{ReturnAsAuraOptions, ReturnFromGraveyardToBattlefieldEffect};

fn resolve_graveyard_return_targets(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target: &ChooseSpec,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let ChooseSpec::Tagged(tag) = target.base() else {
        return resolve_objects_for_effect(game, ctx, target);
    };
    let Some(tagged) = ctx.get_tagged_all(tag) else {
        return Ok(Vec::new());
    };

    let mut ids = Vec::with_capacity(tagged.len());
    for snapshot in tagged {
        let Some(object) = game.object(snapshot.object_id) else {
            return Err(ExecutionError::InvalidTarget);
        };
        if object.stable_id != snapshot.stable_id || object.zone != Zone::Graveyard {
            return Err(ExecutionError::InvalidTarget);
        }
        ids.push(snapshot.object_id);
    }
    Ok(ids)
}

fn choose_aura_attachment_target(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    options: &ReturnAsAuraOptions,
) -> Result<Option<ObjectId>, ExecutionError> {
    let filter_ctx = ctx.filter_context(game);
    let candidates: Vec<ObjectId> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| options.attachment_filter.matches(object, &filter_ctx, game))
        })
        .collect();

    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() == 1 {
        return Ok(candidates.first().copied());
    }

    let chosen: Vec<ObjectId> = make_decision(
        game,
        ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        ChooseObjectsSpec::new(
            ctx.source,
            format!("Choose {}", options.attachment_filter.description()),
            candidates.clone(),
            1,
            Some(1),
        ),
    );
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }

    Ok(chosen.into_iter().find(|id| candidates.contains(id)))
}

fn returned_aura_modifications(options: &ReturnAsAuraOptions) -> Vec<Modification> {
    let mut modifications = vec![
        Modification::AddCardTypes(vec![CardType::Enchantment]),
        Modification::RemoveCardTypes(vec![
            CardType::Artifact,
            CardType::Battle,
            CardType::Creature,
            CardType::Kindred,
            CardType::Land,
            CardType::Planeswalker,
        ]),
        Modification::AddSubtypes(vec![Subtype::Aura]),
    ];
    if options.remove_all_abilities {
        modifications.push(Modification::RemoveAllAbilities);
    }
    // The returned Aura loses its previous abilities, then gains the enchant
    // ability specified by this effect. Both are ability-layer operations.
    modifications.push(Modification::SetAuraAttachmentFilter(
        crate::object::AuraAttachmentFilter::from(options.attachment_filter.clone()).into(),
    ));

    modifications
}

/// Effect that returns a target card from a graveyard to the battlefield.
///
/// This is used for reanimation spells like Animate Dead, Reanimate, etc.
///
/// # Fields
///
/// * `target` - Which card to return
/// * `tapped` - Whether the permanent enters tapped
///
/// # Example
///
/// ```ignore
/// // Return target creature card from your graveyard to the battlefield
/// let effect = ReturnFromGraveyardToBattlefieldEffect::new(
///     ChooseSpec::creature_card_in_graveyard(),
///     false
/// );
/// ```
impl EffectExecutor for ReturnFromGraveyardToBattlefieldEffect {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::with_objects(Vec::new()),
            ));
        }
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                if matches!(self.target.base(), ChooseSpec::Source)
                    && crate::effects::helpers::resolve_source_object_id(game, ctx).is_none()
                {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                let target_ids = match resolve_graveyard_return_targets(game, ctx, &self.target) {
                    Ok(selected) => selected,
                    Err(ExecutionError::InvalidTarget)
                        if !self.target.is_target()
                            && matches!(self.target.base(), ChooseSpec::Object(_))
                            && ctx.targets.iter().any(|target| {
                                matches!(target, crate::effects::ResolvedTarget::Object(_))
                            }) =>
                    {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::target_invalid(),
                        ));
                    }
                    Err(error) => return Err(error),
                };
                if target_ids.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }

                let mut memories = Vec::new();
                for target_id in &target_ids {
                    let obj = game
                        .object(*target_id)
                        .ok_or(ExecutionError::ObjectNotFound(*target_id))?;

                    // An ability that functions while its source is exiled ("return
                    // Cosima to the battlefield", granted to the exiled card) returns
                    // the source from exile; the source identity already proves it is
                    // the same object (CR 400.7).
                    let source_in_exile =
                        matches!(self.target.base(), ChooseSpec::Source) && obj.zone == Zone::Exile;
                    if obj.zone != Zone::Graveyard && !source_in_exile {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::target_invalid(),
                        ));
                    }
                    memories.push(Clone::clone(&ObjectSnapshot::from_object(obj, game)));
                }

                let attachment_target = if let Some(as_aura) = &self.as_aura {
                    match choose_aura_attachment_target(game, ctx, as_aura)? {
                        Some(target) => Some(target),
                        None => {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::target_invalid(),
                            ));
                        }
                    }
                } else {
                    None
                };

                let requests = target_ids
                    .iter()
                    .map(|target_id| {
                        resolve_battlefield_entry_counters(
                            game,
                            ctx,
                            *target_id,
                            &self.enters_with_counters,
                        )
                        .map(|initial_counters| {
                            (
                                *target_id,
                                BattlefieldEntryOptions::preserve(self.tapped)
                                    .with_initial_counters(initial_counters)
                                    .with_entry_attachment(
                                        attachment_target.map(AttachmentTarget::Object),
                                    )
                                    .with_entry_modifications(
                                        self.as_aura
                                            .as_ref()
                                            .map(returned_aura_modifications)
                                            .unwrap_or_default(),
                                    ),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                super::execute_battlefield_entries_with_outputs(
                    game,
                    ctx,
                    requests,
                    false,
                    |_, _, receipts| {
                        let mut moved = Vec::new();
                        let chosen_memory = memories.clone();
                        let mut affected_memory = Vec::new();
                        for (receipt, memory) in receipts.iter().zip(memories) {
                            match &receipt.outcome {
                                BattlefieldEntryOutcome::Moved(new_id) => {
                                    moved.push(*new_id);
                                    affected_memory.push(memory);
                                }
                                BattlefieldEntryOutcome::Redirected(change) => {
                                    if !change.new_object_ids.is_empty() {
                                        affected_memory.push(memory);
                                    }
                                    moved.extend(change.new_object_ids.iter().copied());
                                }
                                BattlefieldEntryOutcome::Prevented => {}
                            }
                        }
                        Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            if moved.is_empty() {
                                EffectOutcome::impossible()
                            } else {
                                EffectOutcome::with_objects(moved)
                                    .with_affected_object_memory(affected_memory)
                                    .with_chosen_object_memory(chosen_memory)
                            },
                        ))
                    },
                )
                .map(|commit| commit.outcome)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        Some(self.target.count())
    }

    fn target_description(&self) -> &'static str {
        "card in graveyard to return"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    struct SelectIdsDecisionMaker {
        chosen: Vec<ObjectId>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chosen
                .iter()
                .copied()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    #[test]
    fn test_reanimate_creature_untapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with new ID
        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and untapped
            assert!(game.battlefield.contains(&new_id));
            assert!(!game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
        // Graveyard should be empty
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn dynamic_x_entry_counters_are_part_of_the_return_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Awakened Relic", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)])
            .with_x(3);
        let effect = ReturnFromGraveyardToBattlefieldEffect::creature().with_entry_counter(
            ironsmith_core::BattlefieldEntryCounterSpec::new(
                crate::CounterType::PlusOnePlusOne,
                crate::effect::Value::X,
                ironsmith_core::BattlefieldEntryCounterSurface::Inline,
            ),
        );

        let result = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected returned object");
        };
        assert_eq!(ids.len(), 1);
        assert_eq!(
            game.counter_count(ids[0], crate::CounterType::PlusOnePlusOne),
            3
        );
    }

    #[test]
    fn test_reanimate_creature_tapped() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature_tapped();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should return Objects with new ID
        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature should be on battlefield and tapped
            assert!(game.battlefield.contains(&new_id));
            assert!(game.is_tapped(new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_reanimate_non_targeted_choice_uses_selected_graveyard_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature_in_graveyard(&mut game, "First", alice);
        let second = create_creature_in_graveyard(&mut game, "Second", alice);
        let source = game.new_object_id();
        let mut dm = SelectIdsDecisionMaker {
            chosen: vec![second],
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        let returned_name = game
            .object(ids[0])
            .map(|obj| obj.name.to_string())
            .expect("reanimated permanent should exist");
        assert_eq!(returned_name, "Second");
        assert!(game.players[0].graveyard.contains(&first));
    }

    #[test]
    fn test_return_as_aura_attaches_to_legal_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Returned Aura", alice);
        game.object_mut(creature_id)
            .expect("returned card exists")
            .abilities_mut()
            .push(crate::ability::Ability {
                kind: crate::ability::AbilityKind::Static(
                    crate::static_abilities::StaticAbility::indestructible(),
                ),
                functional_zones: vec![Zone::Battlefield],
            });
        let bear = create_creature_on_battlefield(&mut game, "Bear", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature()
            .as_aura_removing_all_abilities(crate::filter::ObjectFilter::creature().you_control());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        let returned = ids[0];
        assert_eq!(
            game.object(returned).and_then(|object| object.attached_to),
            Some(AttachmentTarget::Object(bear))
        );
        assert!(game.current_has_card_type(returned, CardType::Enchantment));
        assert!(!game.current_has_card_type(returned, CardType::Creature));
        assert!(game.current_has_subtype(returned, Subtype::Aura));
        assert!(!game.current_has_static_ability_id(
            returned,
            crate::static_abilities::StaticAbilityId::Indestructible
        ));
    }

    #[test]
    fn test_return_as_aura_without_legal_attachment_stays_in_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Returned Aura", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature()
            .as_aura(crate::filter::ObjectFilter::creature().you_control());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert!(game.players[0].graveyard.contains(&creature_id));
        assert!(!game.battlefield.contains(&creature_id));
    }

    #[test]
    fn test_reanimate_multiple_non_targeted_choices() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature_in_graveyard(&mut game, "First", alice);
        let second = create_creature_in_graveyard(&mut game, "Second", alice);
        let third = create_creature_in_graveyard(&mut game, "Third", alice);
        let source = game.new_object_id();
        let mut dm = SelectIdsDecisionMaker {
            chosen: vec![first, third],
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let mut filter = crate::filter::ObjectFilter::creature();
        filter.zone = Some(Zone::Graveyard);

        let effect = ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(2)),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 2);
        let returned_names = ids
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.as_str()))
            .collect::<Vec<_>>();
        assert!(returned_names.contains(&"First"));
        assert!(returned_names.contains(&"Third"));
        assert!(game.players[0].graveyard.contains(&second));
    }

    #[test]
    fn test_reanimate_from_wrong_zone_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        // Create creature on battlefield, not in graveyard
        let creature_id = create_creature_on_battlefield(&mut game, "Grizzly Bears", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should fail - target not in graveyard
        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_reanimate_opponent_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature_id = create_creature_in_graveyard(&mut game, "Massacre Wurm", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Should succeed
        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let new_id = ids[0];
            // Creature enters under owner's (Bob's) control by default
            // (Reanimate effects that give you control need additional logic)
            assert!(game.battlefield.contains(&new_id));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_reanimate_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let result = effect.execute(&mut game, &mut ctx);

        // Should return error - no target
        assert!(result.is_err());
    }

    #[test]
    fn test_reanimate_tagged_target_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_in_graveyard(&mut game, "Griselbrand", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);
        ctx.tag_object("reanimate_target", snapshot);

        let effect = ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::Tagged("reanimate_target".into()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        assert!(game.battlefield.contains(&ids[0]));
    }

    #[test]
    fn test_reanimate_tagged_target_with_stale_object_id_fails() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_on_battlefield(&mut game, "Griselbrand", alice);
        let mut snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );

        let first_graveyard_id = game
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("move to graveyard should succeed");
        assert_ne!(first_graveyard_id, creature_id);
        snapshot.object_id = first_graveyard_id;

        let interim_battlefield_id = game
            .move_object_by_effect(first_graveyard_id, Zone::Battlefield)
            .expect("move back to battlefield should succeed");
        let second_graveyard_id = game
            .move_object_by_effect(interim_battlefield_id, Zone::Graveyard)
            .expect("move back to graveyard should succeed");
        assert_ne!(second_graveyard_id, first_graveyard_id);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("reanimate_target", snapshot);

        let effect = ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::Tagged("reanimate_target".into()),
            false,
        );
        let result = effect.execute(&mut game, &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_reanimate_clone_box() {
        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ReturnFromGraveyardToBattlefieldEffect"));
    }

    struct RandomOnly;
    impl DecisionMaker for RandomOnly {
        fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> {
            panic!("random returns must not delegate object selection to a player");
        }
    }

    fn random_return(count: crate::effect::ChoiceCount) -> ReturnFromGraveyardToBattlefieldEffect {
        ReturnFromGraveyardToBattlefieldEffect::new(
            ChooseSpec::Object(
                crate::filter::ObjectFilter::creature()
                    .in_zone(Zone::Graveyard)
                    .owned_by(crate::target::PlayerFilter::You),
            ).with_count(count.at_random()),
            false,
        )
    }

    #[test]
    fn random_return_samples_without_replacement_and_clamps_empty_or_insufficient_pools() {
        for dynamic in [false, true] {
            for size in [0, 1, 4] {
                let mut game = setup_game();
                let alice = PlayerId(0);
                let source = game.new_object_id();
                for index in 0..size {
                    create_creature_in_graveyard(&mut game, &format!("Eligible {index}"), alice);
                }
                let foreign = create_creature_in_graveyard(&mut game, "Foreign", PlayerId(1));
                let before = game.irreversible_random_count();
                let count = if dynamic { crate::effect::ChoiceCount::dynamic_x() }
                    else { crate::effect::ChoiceCount::exactly(2) };
                let mut dm = RandomOnly;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_x(2);
                let outcome = random_return(count).execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.battlefield.len(), size.min(2));
                assert_eq!(game.players[0].graveyard.len(), size.saturating_sub(2));
                assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
                assert_eq!(game.irreversible_random_count(), before + u64::from(size > 0));
                if size > 0 {
                    let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
                        panic!("expected successful return identities");
                    };
                    assert_eq!(ids.len(), size.min(2));
                    assert_eq!(ids.iter().copied().collect::<std::collections::HashSet<_>>().len(), ids.len());
                }
            }
        }
    }

    #[test]
    fn random_return_rolls_back_random_authority_and_objects_on_later_value_error() {
        let mut game = setup_game();
        let alice = PlayerId(0);
        let source = game.new_object_id();
        let original = create_creature_in_graveyard(&mut game, "Eligible", alice);
        game.queue_transcript_random_seeds([9876]);
        let before_seed = game.random_seed();
        let before_count = game.irreversible_random_count();
        let before_ids = game.next_object_id_counter();
        let effect = random_return(crate::effect::ChoiceCount::exactly(1))
            .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
                crate::CounterType::PlusOnePlusOne,
                crate::effect::Value::X,
                ironsmith_core::BattlefieldEntryCounterSurface::Inline,
            ));
        let mut dm = RandomOnly;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        assert!(matches!(effect.execute(&mut game, &mut ctx), Err(ExecutionError::UnresolvableValue(_))));
        assert_eq!(game.random_seed(), before_seed);
        assert_eq!(game.irreversible_random_count(), before_count);
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
        assert!(game.battlefield.is_empty());
        // A retry consumes the same queued authority, after supplying X.
        ctx.x_value = Some(3);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else { panic!("expected return") };
        assert_eq!(game.random_seed(), 9876);
        assert_eq!(game.irreversible_random_count(), before_count + 1);
        assert_eq!(game.counter_count(ids[0], crate::CounterType::PlusOnePlusOne), 3);
    }

    #[test]
    fn random_return_pending_entry_choice_does_not_commit_the_draw_or_move() {
        struct PauseColor { pending: bool, pause: bool }
        impl DecisionMaker for PauseColor {
            fn decide_colors(&mut self, _: &GameState, _: &crate::decisions::context::ColorsContext) -> Vec<crate::color::Color> {
                self.pending = self.pause;
                vec![crate::color::Color::Blue]
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let mut game = setup_game();
        let alice = PlayerId(0);
        let source = game.new_object_id();
        let original = create_creature_in_graveyard(&mut game, "Color entrant", alice);
        game.object_mut(original).unwrap().abilities_mut().push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        game.queue_transcript_random_seeds([4567]);
        let before_ids = game.next_object_id_counter();
        let before_seed = game.random_seed();
        let before_count = game.irreversible_random_count();
        let mut dm = PauseColor { pending: false, pause: true };
        let effect = random_return(crate::effect::ChoiceCount::exactly(1));
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
        }
        assert!(dm.pending);
        assert_eq!(game.object(original).unwrap().zone, Zone::Graveyard);
        assert!(game.battlefield.is_empty());
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert_eq!(game.random_seed(), before_seed);
        assert_eq!(game.irreversible_random_count(), before_count);
        dm.pending = false;
        dm.pause = false;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else { panic!("expected return") };
        assert_eq!(game.chosen_color(ids[0]), Some(crate::color::Color::Blue));
        assert_eq!(game.random_seed(), 4567);
        assert_eq!(game.irreversible_random_count(), before_count + 1);
    }

    #[test]
    fn test_reanimate_get_target_spec() {
        let effect = ReturnFromGraveyardToBattlefieldEffect::creature();
        assert!(effect.get_target_spec().is_some());
    }
}

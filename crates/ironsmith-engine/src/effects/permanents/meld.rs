//! Meld effect implementation.

use crate::combat_state::{AttackerInfo, get_attack_target};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome,
    finish_battlefield_entry_receipts_with_outputs, move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::game_state::MeldComponentState;
use crate::object::ObjectKind;
use crate::zone::Zone;
pub type MeldEffect = ironsmith_core::MeldEffect;

fn current_source_id(game: &GameState, ctx: &ExecutionContext) -> Option<crate::ids::ObjectId> {
    if game.object(ctx.source).is_some() {
        return Some(ctx.source);
    }
    ctx.source_snapshot
        .as_ref()
        .and_then(|snapshot| game.find_object_by_stable_id(snapshot.stable_id))
}

fn exile_meld_components(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    objects: [crate::ids::ObjectId; 2],
) -> Result<
    (
        Option<(crate::ids::ObjectId, crate::ids::ObjectId)>,
        crate::effects::CompletedEffectOutputs,
    ),
    ExecutionError,
> {
    use crate::events::processing::{
        EventOutcome, PreparedEventOutcome, ReplacementEventContext,
        commit_prepared_zone_change_with_outputs, prepare_zone_change_scoped,
    };
    let snapshots = objects
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|object| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    object, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if snapshots.len() != 2 {
        return Ok((
            None,
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()),
        ));
    }
    let lookback = game.trigger_source_lookback_snapshots();
    let additional = ctx.additional_replacement_effects_snapshot();
    let opened_batch = game.open_simultaneous_action();
    let prepared = (|| -> Result<_, ExecutionError> {
        let mut proposals = Vec::with_capacity(2);
        for (&object, snapshot) in objects.iter().zip(&snapshots) {
            let scope = ReplacementEventContext::with_scope(
                &game,
                crate::events::Event::zone_change(
                    object,
                    Zone::Battlefield,
                    Zone::Exile,
                    ctx.cause.clone(),
                    Some(snapshot.clone()),
                )
                .with_provenance(ctx.provenance),
                &ctx.replacement,
            );
            proposals.push(prepare_zone_change_scoped(
                game,
                object,
                Zone::Battlefield,
                Zone::Exile,
                ctx.cause.clone(),
                ctx.decision_maker,
                &additional,
                Some(snapshot.clone()),
                Some(&scope),
                Vec::new(),
                Some(&lookback),
            )?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
        }
        let mut published_outputs = Vec::new();
        let mut receipts = Vec::new();
        let mut arrivals = Vec::new();
        for (
            object,
            PreparedEventOutcome {
                original,
                mut programs,
            },
        ) in objects.into_iter().zip(proposals)
        {
            let original = match original {
                EventOutcome::Proceed(proposal) => {
                    if game
                        .object(object)
                        .is_some_and(|card| card.zone == Zone::Battlefield)
                    {
                        let committed = commit_prepared_zone_change_with_outputs(
                            game,
                            object,
                            proposal,
                            ctx.decision_maker,
                        )?;
                        published_outputs.extend(committed.published_outputs);
                        let mut committed = committed.receipt;
                        programs.append(&mut committed.programs);
                        committed.original
                    } else {
                        EventOutcome::NotApplicable
                    }
                }
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            let ids = match &original {
                EventOutcome::Proceed(id) => {
                    let mut ids = game.take_zone_change_results(object);
                    if ids.is_empty() {
                        ids.push(*id);
                    }
                    game.record_zone_change_results(object, ids.clone());
                    ids
                }
                EventOutcome::Replaced => {
                    let ids = game.take_zone_change_results(object);
                    if !ids.is_empty() {
                        game.record_zone_change_results(object, ids.clone());
                    }
                    ids
                }
                _ => Vec::new(),
            };
            arrivals.push(ids.first().copied());
            let original = match original {
                EventOutcome::Proceed(id) => {
                    let final_zone = game.object(id).map(|card| card.zone).ok_or_else(|| {
                        ExecutionError::InternalError(
                            "meld departure receipt lost its arrival".into(),
                        )
                    })?;
                    EventOutcome::Proceed(crate::effects::zones::AppliedZoneChange {
                        final_zone,
                        new_object_id: Some(id),
                        new_object_ids: ids,
                    })
                }
                EventOutcome::Prevented => EventOutcome::Prevented,
                EventOutcome::Replaced => EventOutcome::Replaced,
                EventOutcome::NotApplicable => EventOutcome::NotApplicable,
            };
            receipts.push((object, PreparedEventOutcome { original, programs }));
        }
        Ok(Some((receipts, arrivals, published_outputs)))
    })();
    game.close_simultaneous_action(opened_batch);
    let Some((receipts, arrivals, published_outputs)) = prepared? else {
        return Ok((
            None,
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    };
    let original = EffectOutcome::with_objects(arrivals.iter().flatten().copied().collect())
        .with_affected_object_memory(snapshots.iter().map(Clone::clone).collect());
    // Exile is the first instruction. Complete its added programs before
    // the following meld instruction, keeping the exact original arrivals.
    let mut original = crate::effects::CompletedEffectOutputs::aggregate_only(original);
    original.retain_published_references(published_outputs);
    let outcome = crate::effects::zones::finish_zone_change_receipts_with_outputs(
        game, ctx, original, receipts,
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok((None, outcome));
    }
    Ok((arrivals[0].zip(arrivals[1]), outcome))
}

fn execute_meld_inner(
    effect: &MeldEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let Some(source_id) = current_source_id(game, ctx) else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };
    let Some(source) = game.object(source_id).cloned() else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };
    if source.owner != ctx.controller
        || game.controller_of(&source) != ctx.controller
        || source.kind != ObjectKind::Card
        || game.is_phased_out(source_id)
    {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    }

    let Some(counterpart_name) = crate::cards::meld_counterpart_name(&source.name) else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };

    let candidates = match source.zone {
        Zone::Battlefield => &game.battlefield,
        Zone::Exile => &game.exile,
        _ => {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }
    };
    let candidates = candidates
        .iter()
        .copied()
        .filter(|&candidate_id| {
            game.object(candidate_id).is_some_and(|candidate| {
                candidate_id != source_id
                    && !game.is_phased_out(candidate_id)
                    && candidate.owner == ctx.controller
                    && game.controller_of(candidate) == ctx.controller
                    && candidate.name.eq_ignore_ascii_case(counterpart_name)
            })
        })
        .collect::<Vec<_>>();
    let Some(&first) = candidates.first() else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };
    let counterpart_id = if candidates.len() == 1 {
        first
    } else {
        let spec = crate::decisions::specs::ChooseObjectsSpec::new(
            ctx.source,
            "Choose a permanent to meld with this permanent",
            candidates.clone(),
            1,
            Some(1),
        );
        let chosen = crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        crate::effects::helpers::normalize_object_selection(chosen, &candidates, 1)[0]
    };

    let source_attack_target = if effect.enters_attacking {
        game.combat
            .as_ref()
            .and_then(|combat| get_attack_target(combat, source_id).cloned())
    } else {
        None
    };

    let (arrivals, departure_outcome) = if source.zone == Zone::Battlefield {
        exile_meld_components(game, ctx, [source_id, counterpart_id])?
    } else {
        (
            Some((source_id, counterpart_id)),
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()),
        )
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let entry_outcome = (|| -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Some((source_exile_id, counterpart_exile_id)) = arrivals else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        };

        let Some(exiled_source) = game.object(source_exile_id) else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        };
        let Some(exiled_counterpart) = game.object(counterpart_exile_id) else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        };
        // CR 701.42b/c: only the two cards of the meld pair can be melded. A
        // permanent that was merely copying the counterpart (a Clone named
        // Gisela) is no longer named that once exiled, so it stays in exile.
        if exiled_source.zone != Zone::Exile
            || exiled_counterpart.zone != Zone::Exile
            || exiled_source.kind != ObjectKind::Card
            || exiled_counterpart.kind != ObjectKind::Card
            || crate::cards::meld_counterpart_name(&exiled_source.name)
                .is_none_or(|name| !name.eq_ignore_ascii_case(counterpart_name))
            || !exiled_counterpart
                .name
                .eq_ignore_ascii_case(counterpart_name)
        {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }

        // CR 712.8g: the melded permanent's mana value is the sum of the mana
        // values of its front faces.
        let front_faces_mana_cost = crate::mana::ManaCost::from_pips(
            exiled_source
                .mana_cost
                .iter()
                .chain(exiled_counterpart.mana_cost.iter())
                .flat_map(|cost| cost.pips().iter().cloned())
                .collect(),
        );

        let Some(result_def) =
            game.linked_face_definition_by_name_or_id(Some(&effect.result_name), None)
        else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        };

        let meld_components = vec![
            MeldComponentState {
                stable_id: exiled_source.stable_id,
                owner: exiled_source.owner,
                name: exiled_source.name.to_string(),
            },
            MeldComponentState {
                stable_id: exiled_counterpart.stable_id,
                owner: exiled_counterpart.owner,
                name: exiled_counterpart.name.to_string(),
            },
        ];

        let entry = move_to_battlefield_with_options(
            game,
            ctx,
            source_exile_id,
            BattlefieldEntryOptions::specific(ctx.controller, effect.enters_tapped)
                .with_composite_entry(result_def, vec![source_exile_id, counterpart_exile_id])
                .with_linked_face_mana_cost(front_faces_mana_cost),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let receipt = entry.ok_or_else(|| {
            ExecutionError::InternalError("meld entry has no completed receipt".into())
        })?;
        let original = match &receipt.outcome {
            BattlefieldEntryOutcome::Moved(new_id) => {
                let new_id = *new_id;
                game.set_melded_permanent(new_id, meld_components);
                if let Some(target) = source_attack_target
                    && let Some(combat) = game.combat.as_mut()
                {
                    combat.attackers.push(AttackerInfo {
                        creature: new_id,
                        target,
                    });
                }
                EffectOutcome::with_objects(vec![new_id])
            }
            BattlefieldEntryOutcome::Redirected(change) => {
                EffectOutcome::with_objects(change.new_object_ids.clone())
            }
            BattlefieldEntryOutcome::Prevented => EffectOutcome::resolved(),
        };
        finish_battlefield_entry_receipts_with_outputs(game, ctx, original, vec![receipt])
    })()?;
    let mut aggregate = departure_outcome.outcome.clone();
    aggregate
        .events
        .extend(entry_outcome.outcome.events.iter().cloned());
    aggregate
        .execution_facts
        .extend(entry_outcome.outcome.execution_facts.iter().cloned());
    aggregate.status = entry_outcome.outcome.status;
    aggregate.value = entry_outcome.outcome.value.clone();
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        [departure_outcome, entry_outcome],
        |_| aggregate,
    ))
}

impl EffectExecutor for MeldEffect {
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
                EffectOutcome::count(0),
            ));
        }
        crate::effects::composition::execute_checkpoint_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_meld_inner(self, game, ctx),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::{CardDefinition, register_runtime_custom_card};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{Object, ObjectKind};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ChooseSpec;
    use crate::types::{CardType, Subtype};

    fn card_definition(name: &str, types: Vec<CardType>, pt: Option<(i32, i32)>) -> CardDefinition {
        let mut builder = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(types)
            .oracle_text("");
        if let Some((power, toughness)) = pt {
            builder = builder
                .subtypes(vec![Subtype::Rat])
                .power_toughness(PowerToughness::fixed(power, toughness));
        }
        CardDefinition::new(builder.build())
    }

    fn register_test_meld_cards() {
        register_runtime_custom_card(card_definition(
            "Graf Rats",
            vec![CardType::Creature],
            Some((2, 1)),
        ));
        register_runtime_custom_card(card_definition(
            "Midnight Scavengers",
            vec![CardType::Creature],
            Some((3, 3)),
        ));
        register_runtime_custom_card(card_definition(
            "Chittering Host",
            vec![CardType::Creature],
            Some((5, 6)),
        ));
    }

    fn create_test_melded_permanent(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            owner,
            Zone::Exile,
        );
        game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            owner,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, owner);
        MeldEffect::new("Chittering Host")
            .execute(game, &mut ctx)
            .expect("meld should resolve")
            .first_output_object()
            .expect("meld should produce a result object")
    }

    fn names_for_ids(game: &GameState, ids: &[ObjectId]) -> Vec<String> {
        ids.iter()
            .map(|id| game.object(*id).expect("object exists").name.to_string())
            .collect()
    }

    fn library_top_to_bottom_names(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player exists")
            .library
            .iter()
            .rev()
            .map(|id| {
                game.object(*id)
                    .expect("library object exists")
                    .name
                    .to_string()
            })
            .collect()
    }

    #[derive(Default)]
    struct ReverseOrderDecisionMaker {
        prompts: Vec<String>,
        returned_order: Vec<ObjectId>,
    }

    impl DecisionMaker for ReverseOrderDecisionMaker {
        fn decide_order(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.prompts.push(ctx.description.clone());
            let mut ids = ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            ids.reverse();
            self.returned_order = ids.clone();
            ids
        }
    }

    #[test]
    fn meld_effect_creates_result_from_exiled_pair() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        let created = outcome.output_objects();
        assert!(!created.is_empty(), "meld should produce a result object");
        assert_eq!(created.len(), 1);
        let result = game.object(created[0]).expect("meld result should exist");
        assert_eq!(result.zone, Zone::Battlefield);
        assert_eq!(result.name, "Chittering Host");
        assert!(
            game.object(source).is_none(),
            "source card should be consumed"
        );
        assert!(
            game.object(counterpart).is_none(),
            "counterpart card should be consumed"
        );
    }

    #[test]
    fn meld_effect_leaves_objects_exiled_when_pair_is_invalid() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );

        let token_id = game.new_object_id();
        let token_card = CardBuilder::new(CardId::new(), "Midnight Scavengers")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rat])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
        let mut token = Object::from_card(token_id, &token_card, alice, Zone::Exile);
        token.kind = ObjectKind::Token;
        game.add_object(token);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        assert!(
            outcome
                .affected_objects()
                .is_none_or(|objects| objects.is_empty())
        );
        assert_eq!(
            game.object(source).expect("source should remain").zone,
            Zone::Exile
        );
        assert!(game.battlefield.iter().copied().all(|id| {
            game.object(id)
                .is_none_or(|obj| obj.name != "Chittering Host")
        }));
    }

    #[test]
    fn meld_effect_can_enter_tapped_and_attacking() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_battlefield = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Battlefield,
        );
        let counterpart_battlefield = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Battlefield,
        );
        let source_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source_battlefield).expect("source exists"),
            &game,
        );
        game.combat = Some(CombatState {
            block_declaration_complete: true,
            attacked_permanent_types: Default::default(),
        last_attack_declaration_step_players: None,
            attackers: vec![
                AttackerInfo {
                    creature: source_battlefield,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: counterpart_battlefield,
                    target: AttackTarget::Player(bob),
                },
            ],
            blockers: Default::default(),
            damage_assignment_order: Default::default(),
            attacking_bands: Default::default(),
            blocked_attackers: Default::default(),
            had_to_attack_this_combat: Default::default(),
        });

        let mut ctx = ExecutionContext::new_default(source_battlefield, alice)
            .with_source_snapshot(source_snapshot);
        let outcome = MeldEffect::new("Chittering Host")
            .enters_tapped(true)
            .enters_attacking(true)
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");

        let result_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");
        assert!(game.is_tapped(result_id), "meld result should enter tapped");
        let attackers = &game.combat.as_ref().expect("combat should exist").attackers;
        assert!(attackers.iter().any(|info| {
            info.creature == result_id && info.target == AttackTarget::Player(bob)
        }));
    }

    #[test]
    fn melded_permanent_leaves_battlefield_as_two_front_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );
        let source_stable = game.object(source).expect("source exists").stable_id;
        let counterpart_stable = game
            .object(counterpart)
            .expect("counterpart exists")
            .stable_id;

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");
        let melded_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");

        let first_graveyard_id = game
            .move_object_by_effect(melded_id, Zone::Graveyard)
            .expect("melded permanent should move");
        let mut moved_ids = game.take_zone_change_results(melded_id);
        if moved_ids.is_empty() {
            moved_ids.push(first_graveyard_id);
        }

        assert_eq!(moved_ids.len(), 2, "meld should split into two cards");
        let moved_names: Vec<_> = moved_ids
            .iter()
            .map(|&id| game.object(id).expect("moved card exists").name.to_string())
            .collect();
        assert!(moved_names.contains(&"Graf Rats".to_string()));
        assert!(moved_names.contains(&"Midnight Scavengers".to_string()));
        assert!(
            moved_ids.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.stable_id == source_stable)),
            "one split card should preserve Graf Rats stable identity"
        );
        assert!(
            moved_ids.iter().any(|&id| game
                .object(id)
                .is_some_and(|obj| obj.stable_id == counterpart_stable)),
            "one split card should preserve Midnight Scavengers stable identity"
        );
        assert!(
            game.battlefield.iter().all(|&id| game
                .object(id)
                .is_none_or(|obj| obj.name != "Chittering Host")),
            "meld result should leave the battlefield"
        );
    }

    #[test]
    fn melded_permanent_zone_change_event_includes_split_result_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                .expect("source definition"),
            alice,
            Zone::Exile,
        );
        let _counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .expect("counterpart definition"),
            alice,
            Zone::Exile,
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("meld should resolve");
        let melded_id = outcome
            .first_output_object()
            .expect("meld should produce a result object");

        game.move_object_by_effect(melded_id, Zone::Graveyard)
            .expect("melded permanent should move");
        let pending = game.take_pending_trigger_events();
        let zone_change = pending
            .iter()
            .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
            .find(|event| event.from == Zone::Battlefield && event.to == Zone::Graveyard)
            .expect("zone change event should be queued");

        assert_eq!(zone_change.objects, vec![melded_id]);
        assert_eq!(zone_change.result_objects.len(), 2);
        assert_eq!(zone_change.from, Zone::Battlefield);
        assert_eq!(zone_change.to, Zone::Graveyard);
    }

    #[test]
    fn destroy_uses_order_prompt_for_split_graveyard_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect =
            crate::effects::zones::DestroyEffect::with_spec(ChooseSpec::SpecificObject(melded_id));
        effect
            .execute(&mut game, &mut ctx)
            .expect("destroy should resolve");

        let graveyard = &game.player(alice).expect("alice exists").graveyard;
        assert_eq!(
            graveyard, &dm.returned_order,
            "graveyard order should follow the chooser's ordering prompt",
        );
        let mut graveyard_names = names_for_ids(&game, graveyard);
        graveyard_names.sort();
        assert_eq!(
            graveyard_names,
            vec!["Graf Rats".to_string(), "Midnight Scavengers".to_string()]
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("split cards in the graveyard")),
            "destroying a melded permanent should prompt for graveyard order",
        );
    }

    #[test]
    fn exile_uses_order_prompt_for_split_exile_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        crate::effects::zones::ExileEffect::specific(melded_id)
            .execute(&mut game, &mut ctx)
            .expect("exile should resolve");

        let exile_names = names_for_ids(&game, &game.exile);
        assert_eq!(
            exile_names,
            vec!["Midnight Scavengers".to_string(), "Graf Rats".to_string()],
            "exile order should follow the chooser's ordering prompt",
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("split cards in exile")),
            "exiling a melded permanent should prompt for exile order",
        );
    }

    #[test]
    fn move_to_library_uses_order_prompt_for_split_library_cards() {
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let melded_id = create_test_melded_permanent(&mut game, alice);
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Library Bottom")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Existing Top")
                .card_types(vec![CardType::Instant])
                .build(),
            alice,
            Zone::Library,
        );
        let source = game.new_object_id();

        let mut dm = ReverseOrderDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        crate::effects::zones::MoveToZoneEffect::to_top_of_library(ChooseSpec::SpecificObject(
            melded_id,
        ))
        .execute(&mut game, &mut ctx)
        .expect("move to library should resolve");

        let library_names = library_top_to_bottom_names(&game, alice);
        assert_eq!(
            library_names,
            vec![
                "Graf Rats".to_string(),
                "Midnight Scavengers".to_string(),
                "Existing Top".to_string(),
                "Library Bottom".to_string(),
            ],
            "library top-to-bottom order should follow the chooser's ordering prompt",
        );
        assert!(
            dm.prompts
                .iter()
                .any(|prompt| prompt.contains("top card among them")),
            "moving a melded permanent into a library should prompt for library order",
        );
    }

    #[test]
    fn meld_entry_redirect_preserves_two_physical_cards_without_a_provisional_third_card() {
        register_test_meld_cards();
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Exile,
        );
        let source_stable = game.object(source).unwrap().stable_id;
        let counterpart_stable = game.object(counterpart).unwrap().stable_id;
        let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::default(),
                    None,
                    Some(Zone::Battlefield),
                ),
                crate::replacement::ReplacementAction::ChangeDestination(Zone::Exile),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.battlefield.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_none()
        );
        // Redirecting the entry cannot create a third physical card from the
        // transient combined-result representation.
        assert_eq!(game.exile.len(), 2);
        let source = game.find_object_by_stable_id(source_stable).unwrap();
        let counterpart = game.find_object_by_stable_id(counterpart_stable).unwrap();
        assert_eq!(game.object(source).unwrap().zone, Zone::Exile);
        assert_eq!(game.object(counterpart).unwrap().zone, Zone::Exile);
        assert_eq!(game.objects_in_deterministic_order().len(), 2);
        assert!(game.command_zone.is_empty());
        assert_eq!(
            names_for_ids(&game, game.exile.as_slice()),
            vec!["Graf Rats".to_string(), "Midnight Scavengers".to_string()]
        );
    }

    #[test]
    fn meld_entry_source_exile_honors_zone_replacement_before_combining_cards() {
        register_test_meld_cards();
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Battlefield,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Battlefield,
        );
        let source_stable = game.object(source).unwrap().stable_id;
        let counterpart_stable = game.object(counterpart).unwrap().stable_id;
        let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(source),
                    Some(Zone::Battlefield),
                    Some(Zone::Exile),
                ),
                crate::replacement::ReplacementAction::ChangeDestination(Zone::Hand),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.find_object_by_stable_id(source_stable)
                .and_then(|id| game.object(id).map(|object| object.zone)),
            Some(Zone::Hand)
        );
        assert_eq!(
            game.find_object_by_stable_id(counterpart_stable)
                .and_then(|id| game.object(id).map(|object| object.zone)),
            Some(Zone::Exile)
        );
        assert!(game.battlefield.is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(redirect)
                .is_none()
        );
        assert_eq!(game.objects_in_deterministic_order().len(), 2);
        assert!(game.command_zone.is_empty());
    }

    #[test]
    fn meld_entry_program_error_restores_physical_cards_and_provisional_objects() {
        register_test_meld_cards();
        for original_zone in [Zone::Battlefield, Zone::Exile] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                    .unwrap(),
                alice,
                original_zone,
            );
            let counterpart = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(
                    Some("Midnight Scavengers"),
                    None,
                )
                .unwrap(),
                alice,
                original_zone,
            );
            let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        crate::target::ObjectFilter::default(),
                    ),
                    crate::replacement::ReplacementAction::AsEntersProgram(
                        crate::resolution::ResolutionProgram::from_effects(vec![
                            crate::effect::Effect::gain_life(2),
                            crate::effect::Effect::lose_life(crate::effect::Value::X),
                        ]),
                    ),
                ),
            );
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let result = MeldEffect::new("Chittering Host").execute(&mut game, &mut ctx);
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.object(source).map(|object| object.zone),
                Some(original_zone)
            );
            assert_eq!(
                game.object(counterpart).map(|object| object.zone),
                Some(original_zone)
            );
            assert_eq!(game.objects_in_deterministic_order().len(), 2);
            assert!(game.command_zone.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(replacement)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn meld_preliminary_exiles_honor_independent_prevention_and_redirect() {
        register_test_meld_cards();
        for prevent_source in [false, true] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                    .unwrap(),
                alice,
                Zone::Battlefield,
            );
            let counterpart = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(
                    Some("Midnight Scavengers"),
                    None,
                )
                .unwrap(),
                alice,
                Zone::Battlefield,
            );
            let source_stable = game.object(source).unwrap().stable_id;
            let counterpart_stable = game.object(counterpart).unwrap().stable_id;
            let source_replacement = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(source),
                        Some(Zone::Battlefield),
                        Some(Zone::Exile),
                    ),
                    if prevent_source {
                        crate::replacement::ReplacementAction::Prevent
                    } else {
                        crate::replacement::ReplacementAction::ChangeDestination(Zone::Hand)
                    },
                ),
            );
            let counterpart_replacement = game
                .effect_store
                .replacement_effects
                .add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::specific(counterpart),
                        Some(Zone::Battlefield),
                        Some(Zone::Exile),
                    ),
                    crate::replacement::ReplacementAction::ChangeDestination(Zone::Graveyard),
                ));
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            MeldEffect::new("Chittering Host")
                .execute(&mut game, &mut ctx)
                .unwrap();
            let actual_source = game.find_object_by_stable_id(source_stable).unwrap();
            let actual_counterpart = game.find_object_by_stable_id(counterpart_stable).unwrap();
            assert_eq!(
                game.object(actual_source).unwrap().zone,
                if prevent_source {
                    Zone::Battlefield
                } else {
                    Zone::Hand
                }
            );
            assert_eq!(
                game.object(actual_counterpart).unwrap().zone,
                Zone::Graveyard
            );
            assert_eq!(game.objects_in_deterministic_order().len(), 2);
            assert!(game.command_zone.is_empty());
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(source_replacement)
                    .is_none()
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(counterpart_replacement)
                    .is_none()
            );
            let changes = game.take_pending_trigger_events();
            assert_eq!(
                changes
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::ZoneChange)
                    .count(),
                if prevent_source { 1 } else { 2 }
            );
        }
    }

    #[test]
    fn meld_entry_redirect_moves_only_physical_cards_with_real_zone_events() {
        register_test_meld_cards();
        for destination in [Zone::Exile, Zone::Hand, Zone::Graveyard, Zone::Library] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None)
                    .unwrap(),
                alice,
                Zone::Exile,
            );
            let counterpart = game.create_object_from_definition(
                &crate::cards::linked_face_definition_by_name_or_id(
                    Some("Midnight Scavengers"),
                    None,
                )
                .unwrap(),
                alice,
                Zone::Exile,
            );
            let source_stable = game.object(source).unwrap().stable_id;
            let counterpart_stable = game.object(counterpart).unwrap().stable_id;
            let redirect = game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        crate::target::ObjectFilter::default(),
                        None,
                        Some(Zone::Battlefield),
                    ),
                    crate::replacement::ReplacementAction::ChangeDestination(destination),
                ),
            );
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            MeldEffect::new("Chittering Host")
                .execute(&mut game, &mut ctx)
                .unwrap();
            let actual_source = game.find_object_by_stable_id(source_stable).unwrap();
            let actual_counterpart = game.find_object_by_stable_id(counterpart_stable).unwrap();
            assert!(game.battlefield.is_empty());
            assert!(game.command_zone.is_empty());
            assert_eq!(game.objects_in_deterministic_order().len(), 2);
            assert_eq!(game.object(actual_source).unwrap().zone, destination);
            assert_eq!(game.object(actual_counterpart).unwrap().zone, destination);
            assert_eq!(
                game.object(actual_source).unwrap().name.to_string(),
                "Graf Rats"
            );
            assert_eq!(
                game.object(actual_counterpart).unwrap().name.to_string(),
                "Midnight Scavengers"
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(redirect)
                    .is_none()
            );
            let events = game.take_pending_trigger_events();
            assert!(
                events
                    .iter()
                    .all(|event| event.kind() != crate::events::EventKind::EnterBattlefield)
            );
            let changes = events
                .iter()
                .filter_map(|event| {
                    crate::events::downcast_event::<crate::events::zones::ZoneChangeEvent>(
                        event.inner(),
                    )
                })
                .collect::<Vec<_>>();
            if destination == Zone::Exile {
                // CR 400.8: re-exiling renews each object identity even though
                // it is not a zone change. Both physical cards remain in exile.
                assert_ne!(actual_source, source);
                assert_ne!(actual_counterpart, counterpart);
                assert!(
                    changes.is_empty(),
                    "re-exiling creates no physical zone change"
                );
            } else {
                assert!(
                    changes
                        .iter()
                        .all(|change| change.from == Zone::Exile && change.to == destination)
                );
                // Non-battlefield notifications name destination objects;
                // their pre-move snapshots retain the original identities.
                let originals = changes
                    .iter()
                    .flat_map(|change| change.snapshots().iter().map(|snapshot| snapshot.object_id))
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(originals, [source, counterpart].into_iter().collect());
                let results = changes
                    .iter()
                    .flat_map(|change| change.destination_objects().iter().copied())
                    .collect::<std::collections::BTreeSet<_>>();
                assert_eq!(
                    results,
                    [actual_source, actual_counterpart].into_iter().collect()
                );
                let identities = changes
                    .iter()
                    .flat_map(|change| change.snapshots().iter().map(|snapshot| snapshot.stable_id))
                    .collect::<std::collections::HashSet<_>>();
                assert_eq!(
                    identities,
                    [source_stable, counterpart_stable].into_iter().collect()
                );
            }
        }
    }

    #[test]
    fn meld_successful_entry_reports_both_physical_exile_origins() {
        register_test_meld_cards();
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Exile,
        );
        let physical_identities = [
            game.object(source).unwrap().stable_id,
            game.object(counterpart).unwrap().stable_id,
        ]
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .unwrap();
        let permanent = result.first_output_object().unwrap();
        assert_eq!(game.battlefield.as_slice(), &[permanent]);
        assert!(game.exile.is_empty());
        assert!(game.command_zone.is_empty());
        let events = game.take_pending_trigger_events();
        let changes = events
            .iter()
            .filter_map(|event| {
                crate::events::downcast_event::<crate::events::zones::ZoneChangeEvent>(
                    event.inner(),
                )
            })
            .collect::<Vec<_>>();
        assert!(!changes.is_empty());
        assert!(
            changes
                .iter()
                .all(|change| change.from == Zone::Exile && change.to == Zone::Battlefield),
            "the physical cards enter from exile; no synthetic command-zone movement occurred"
        );
        let snapshots = changes
            .iter()
            .flat_map(|change| change.snapshots())
            .collect::<Vec<_>>();
        let originals = snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(originals, [source, counterpart].into_iter().collect());
        let identities = snapshots
            .iter()
            .map(|snapshot| snapshot.stable_id)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(identities, physical_identities);
        let destinations = changes
            .iter()
            .flat_map(|change| change.destination_objects().iter().copied())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(destinations, [permanent].into_iter().collect());
        let entries = events
            .iter()
            .filter_map(|event| {
                crate::events::downcast_event::<crate::events::EnterBattlefieldEvent>(event.inner())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            entries.len(),
            1,
            "two physical cards become one entering permanent"
        );
        assert_eq!(entries[0].object, permanent);
        assert_eq!(entries[0].from, Zone::Exile);
    }

    fn check_meld_entry_mana_value(copy_mode: u8) {
        check_meld_entry_mana_value_and_layer_filter(copy_mode, 0);
    }

    #[test]
    fn copied_meld_mana_value_selects_continuous_effect_until_copy_expires() {
        check_meld_entry_mana_value_and_layer_filter(2, 1);
    }

    #[test]
    fn copied_meld_mana_value_selects_fallback_continuous_filter_until_copy_expires() {
        check_meld_entry_mana_value_and_layer_filter(2, 2);
    }

    fn check_meld_entry_mana_value_and_layer_filter(copy_mode: u8, check_filter: u8) {
        register_test_meld_cards();
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        // A combined back face has no printed mana cost. Keep this override
        // game-local so concurrent fixtures retain their own definitions.
        let result_definition = CardDefinition::new(
            CardBuilder::new(CardId::new(), "Chittering Host")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(5, 6))
                .build(),
        );
        game.register_linked_face_definition(&result_definition);
        let copy_target = create_test_melded_permanent(&mut game, alice);
        let original_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(copy_target).unwrap(),
            &game,
        );
        assert_eq!(original_snapshot.mana_value(), 4);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Exile,
        );
        game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Exile,
        );
        if copy_mode != 0 {
            game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        crate::target::ObjectFilter::default(),
                    ),
                    crate::replacement::ReplacementAction::EnterAsCopy {
                        source: copy_target,
                        enters_tapped: false,
                        copy_duration: (copy_mode == 2).then_some(crate::effect::Until::EndOfTurn),
                        linked_exile_objects: Vec::new(),
                        additional_counters: Vec::new(),
                        name_override: None,
                        added_colors: crate::color::ColorSet::new(),
                        added_card_types: Vec::new(),
                        removes_other_card_types: false,
                        added_supertypes: Vec::new(),
                        removed_supertypes: Vec::new(),
                        added_subtypes: Vec::new(),
                        added_abilities: Vec::new(),
                        set_base_power_toughness: None,
                        copy_followups: Vec::new(),
                    },
                ),
            );
        }
        let mut ctx = ExecutionContext::new_default(source, alice);
        let permanent = MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .unwrap()
            .first_output_object()
            .unwrap();
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(permanent).unwrap(),
            &game,
        );
        // CR202.3c explicitly covers a copy represented by two other meld cards.
        assert_eq!(snapshot.mana_value(), if copy_mode == 0 { 4 } else { 0 });
        assert_eq!(
            crate::filter::object_current_mana_value(&game, permanent),
            if copy_mode == 0 { 4 } else { 0 }
        );
        if check_filter > 0 {
            let mut filter = crate::target::ObjectFilter::default();
            filter.specific = Some(permanent);
            filter.mana_value = Some(crate::filter::Comparison::Equal(0));
            if check_filter == 2 {
                // An any-of clause forces the layered clone fallback.
                filter.any_of = vec![crate::target::ObjectFilter::default()];
            }
            game.effect_store.continuous_effects.add_effect(
                crate::continuous::ContinuousEffect::new(
                    permanent,
                    alice,
                    crate::continuous::EffectTarget::Filter(filter),
                    crate::continuous::Modification::ModifyPowerToughness {
                        power: 3,
                        toughness: 3,
                    },
                ),
            );
            game.refresh_continuous_state();
            assert_eq!(
                game.current_power(permanent),
                Some(8),
                "layer predicate must see copied mana value zero"
            );
        }
        if copy_mode == 2 {
            game.effect_store.continuous_effects.cleanup_end_of_turn();
            game.refresh_continuous_state();
            let restored = ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(permanent).unwrap(),
                &game,
            );
            assert_eq!(restored.mana_value(), 4);
            assert_eq!(
                crate::filter::object_current_mana_value(&game, permanent),
                4
            );
            if check_filter > 0 {
                assert_eq!(
                    game.current_power(permanent),
                    Some(5),
                    "permanent boost stops matching when the copy expires"
                );
            }
        }
    }

    #[test]
    fn meld_entry_without_copy_keeps_combined_front_face_mana_value() {
        check_meld_entry_mana_value(0);
    }

    #[test]
    fn meld_entry_copy_of_melded_permanent_has_zero_mana_value() {
        check_meld_entry_mana_value(1);
    }

    #[test]
    fn meld_entry_temporary_copy_of_melded_permanent_has_zero_mana_value() {
        check_meld_entry_mana_value(2);
    }

    #[derive(Default)]
    struct InsteadChoiceDecisionMaker {
        prompts: usize,
    }

    impl DecisionMaker for InsteadChoiceDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.options.len(), 2);
            self.prompts += 1;
            vec![0]
        }
    }

    #[test]
    fn meld_instead_payload_keeps_completed_component_movement() {
        check_meld_instead_component_movement(false);
    }

    #[test]
    fn chosen_meld_instead_payload_preserves_source_face_and_completed_movement() {
        check_meld_instead_component_movement(true);
    }

    fn check_meld_instead_component_movement(with_choice: bool) {
        register_test_meld_cards();
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Exile,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Exile,
        );
        let source_stable = game.object(source).unwrap().stable_id;
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    crate::target::ObjectFilter::default(),
                ),
                crate::replacement::ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(2),
                    crate::effect::Effect::move_to_zone(
                        ChooseSpec::SpecificObject(source),
                        Zone::Hand,
                        false,
                    ),
                ]),
            ),
        );
        let unchosen = with_choice.then(|| {
            game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    counterpart,
                    alice,
                    crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                        crate::target::ObjectFilter::default(),
                    ),
                    crate::replacement::ReplacementAction::Instead(vec![
                        crate::effect::Effect::gain_life(99),
                    ]),
                ),
            )
        });
        game.take_pending_trigger_events();
        let prior_events = game.turn_store.turn_history.projected_records().count();
        let mut dm = InsteadChoiceDecisionMaker::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        MeldEffect::new("Chittering Host")
            .execute(&mut game, &mut ctx)
            .expect("a completed replacement payload is not a stale entry-commit error");
        drop(ctx);
        assert_eq!(dm.prompts, usize::from(with_choice));
        if let Some(unchosen) = unchosen {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(unchosen)
                    .is_some()
            );
        }
        let actual_source = game.find_object_by_stable_id(source_stable).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(game.object(actual_source).unwrap().zone, Zone::Hand);
        assert_eq!(
            game.object(actual_source).unwrap().name.to_string(),
            "Graf Rats"
        );
        assert_eq!(game.object(counterpart).unwrap().zone, Zone::Exile);
        assert_eq!(
            game.object(counterpart).unwrap().name.to_string(),
            "Midnight Scavengers"
        );
        assert!(game.battlefield.is_empty());
        assert!(game.command_zone.is_empty());
        assert_eq!(game.objects_in_deterministic_order().len(), 2);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(replacement)
                .is_none()
        );
        let events = game
            .turn_store
            .turn_history
            .projected_records()
            .skip(prior_events)
            .map(|record| &record.event)
            .collect::<Vec<_>>();
        assert!(
            events
                .iter()
                .all(|event| event.kind() != crate::events::EventKind::EnterBattlefield)
        );
        let changes = events
            .iter()
            .filter_map(|event| {
                crate::events::downcast_event::<crate::events::zones::ZoneChangeEvent>(
                    event.inner(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].from, Zone::Exile);
        assert_eq!(changes[0].to, Zone::Hand);
        assert_eq!(changes[0].snapshots()[0].object_id, source);
        assert_eq!(changes[0].snapshots()[0].name.to_string(), "Graf Rats");
        assert_eq!(changes[0].destination_objects(), &[actual_source]);
    }

    fn replacement_meld_check(mode: u8) {
        use crate::effect::{Effect, Value};
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        struct Answers {
            originals: [crate::ids::StableId; 2],
            pause: bool,
            pending: bool,
            calls: usize,
            binding: bool,
        }
        impl DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls += 1;
                let ids = self
                    .originals
                    .iter()
                    .map(|stable| game.find_object_by_stable_id(*stable).unwrap())
                    .collect::<Vec<_>>();
                assert!(
                    ids.iter()
                        .all(|id| game.object(*id).unwrap().zone == Zone::Exile),
                    "departure additions must see both originals in exile before meld entry"
                );
                if self.binding {
                    assert_eq!(
                        game.counter_count(ids[0], crate::object::CounterType::PlusOnePlusOne),
                        1
                    );
                    assert_eq!(
                        game.counter_count(ids[1], crate::object::CounterType::PlusOnePlusOne),
                        0
                    );
                }
                self.pending = self.pause;
                !self.pending
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        register_test_meld_cards();
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Graf Rats"), None).unwrap(),
            alice,
            Zone::Battlefield,
        );
        let counterpart = game.create_object_from_definition(
            &crate::cards::linked_face_definition_by_name_or_id(Some("Midnight Scavengers"), None)
                .unwrap(),
            alice,
            Zone::Battlefield,
        );
        let replacement_source = game.create_object_from_definition(
            &card_definition(
                "Meld replacement source",
                vec![CardType::Creature],
                Some((2, 2)),
            ),
            bob,
            Zone::Battlefield,
        );
        let stable = [
            game.object(source).unwrap().stable_id,
            game.object(counterpart).unwrap().stable_id,
        ];
        let source_snapshot = ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let sentinel = ObjectSnapshot::from_object(game.object(counterpart).unwrap(), &game);
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![
                Effect::new(crate::effects::PutCountersEffect::new(
                    crate::object::CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::tagged("it"),
                )),
                Effect::may(vec![Effect::gain_life(0)]),
            ],
            _ => vec![
                Effect::gain_life(3),
                Effect::may(vec![Effect::gain_life(4)]),
            ],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                replacement_source,
                bob,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    crate::target::ObjectFilter::specific(source),
                    Some(Zone::Battlefield),
                    Some(Zone::Exile),
                ),
                ReplacementAction::Additionally(effects),
            ),
        );
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter();
        let mut dm = Answers {
            originals: stable,
            pause: mode == 2,
            pending: false,
            calls: 0,
            binding: mode == 3,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.source_snapshot = Some(source_snapshot);
        ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let effect = MeldEffect::new("Chittering Host");
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        } else if mode == 2 {
            assert!(ctx.decision_maker.awaiting_choice());
        } else {
            let outcome = result.unwrap();
            assert_eq!(outcome.output_objects().len(), 1);
            let joined = outcome.output_objects()[0];
            assert_eq!(game.object(joined).unwrap().name, "Chittering Host");
            assert_eq!(
                game.player(bob).unwrap().life,
                if mode == 3 { 20 } else { 27 }
            );
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.counter_count(joined, crate::object::CounterType::PlusOnePlusOne),
                0,
                "an exile object's counters don't transfer across its next zone change"
            );
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert_eq!(
            ctx.get_tagged_all("it").unwrap()[0].object_id,
            sentinel.object_id
        );
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(counterpart).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 3 {
            assert_eq!(
                dm.calls, 1,
                "the binding inspection must actually run before meld"
            );
        }
        if mode == 2 {
            assert_eq!(dm.calls, 1);
            dm.pause = false;
            dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(outcome.output_objects().len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!ctx.decision_maker.awaiting_choice());
            drop(ctx);
            assert_eq!(dm.calls, 2);
        }
    }
    #[test]
    fn replacement_meld_departure_additions_before_entry() {
        replacement_meld_check(0);
    }
    #[test]
    fn replacement_meld_error_rolls_back_both_phases() {
        replacement_meld_check(1);
    }
    #[test]
    fn replacement_meld_pending_replays_both_phases() {
        replacement_meld_check(2);
    }
    #[test]
    fn replacement_meld_binds_actual_exile_arrival() {
        replacement_meld_check(3);
    }
}

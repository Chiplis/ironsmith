//! ForEach effect implementation.

use crate::filter::ObjectFilterExt as _;
use std::collections::HashSet;

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::events::ShuffleLibraryEvent;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, TaggedOpbjectRelation};
pub type ForEachObject = ironsmith_core::ForEachObject<Effect>;

fn matching_objects(
    effect: &ForEachObject,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<(crate::ids::ObjectId, ObjectSnapshot)> {
    let filter_ctx = ctx.filter_context(game);

    // For "for each ... revealed/exiled/... this way" patterns, the filter can
    // reference tagged cards outside the battlefield.
    let has_only_is_tagged_constraints = !effect.filter.tagged_constraints.is_empty()
        && effect
            .filter
            .tagged_constraints
            .iter()
            .all(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject);

    if has_only_is_tagged_constraints {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for constraint in &effect.filter.tagged_constraints {
            let Some(snapshots) = ctx.get_tagged_all(&constraint.tag) else {
                continue;
            };
            for snapshot in snapshots {
                if seen.insert(snapshot.stable_id) {
                    candidates.push(snapshot.clone());
                }
            }
        }
        candidates
            .into_iter()
            .filter_map(|snapshot| {
                let current_id =
                    crate::effects::helpers::resolve_tagged_object_id(game, ctx, &snapshot);
                let matched_as_lki = effect.filter.matches_snapshot(&snapshot, &filter_ctx, game);
                let matched_current = current_id
                    .and_then(|id| game.object(id))
                    .is_some_and(|object| effect.filter.matches(object, &filter_ctx, game));
                (matched_as_lki || matched_current)
                    .then_some((current_id.unwrap_or(snapshot.object_id), snapshot))
            })
            .collect()
    } else {
        let candidate_ids: Vec<_> = if let Some(zone) = effect.filter.zone {
            game.zone_ids(zone).collect()
        } else {
            game.battlefield.to_vec()
        };
        candidate_ids
            .into_iter()
            .filter_map(|id| {
                game.object(id).and_then(|object| {
                    effect
                        .filter
                        .matches(object, &filter_ctx, game)
                        .then(|| (id, ObjectSnapshot::from_object(object, game)))
                })
            })
            .collect()
    }
}

#[derive(Debug)]
struct ForEachObjectProposal {
    iterations: Vec<Vec<Box<dyn SimultaneousEffectProposal>>>,
}

impl SimultaneousEffectProposal for ForEachObjectProposal {
    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        crate::effects::damage::DamageActionInputs::collect(
            self.iterations
                .iter()
                .flatten()
                .map(|proposal| proposal.damage_action_inputs()),
        )
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let mut bindings = Vec::new();
        for proposal in self.iterations.into_iter().flatten() {
            bindings.push(proposal.bind_damage_action(game, ctx, owner)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::DamageActionBinding::from_outcome(
                    EffectOutcome::count(0),
                ));
            }
        }
        Ok(crate::effects::DamageActionBinding::from_bindings(
            bindings,
            EffectOutcome::aggregate_summing_counts,
        ))
    }
    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.iterations
            .iter()
            .flatten()
            .flat_map(|proposal| proposal.declared_payment_resources())
            .collect()
    }

    fn declared_life_payments(&self) -> Vec<(crate::ids::PlayerId, u32)> {
        self.iterations
            .iter()
            .flatten()
            .flat_map(|proposal| proposal.declared_life_payments())
            .collect()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.prepare_selection(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for proposal in self.iterations.iter_mut().flatten() {
            proposal.seal_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let mut receipts = Vec::new();
        for proposal in self.iterations.into_iter().flatten() {
            receipts.push(proposal.commit_original_with_outputs(game, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
        }
        Ok(super::compose_original_commits_with_projection_outputs(
            receipts,
            Box::new(|outcomes| EffectOutcome::aggregate_summing_counts(outcomes)),
        ))
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::complete_prepared_original(self, game, ctx)
    }

}

struct ObjectIterationPlan {
    matching: Vec<(crate::ids::ObjectId, ObjectSnapshot)>,
    effects: Vec<Effect>,
    shuffle: Option<crate::effects::ShuffleLibraryEffect>,
    owners: Vec<crate::ids::PlayerId>,
}
impl super::iteration_program::SelectedIterationPlan for ObjectIterationPlan {
    fn len(&self) -> usize {
        self.matching.len()
    }
    fn effects(&self) -> &[Effect] {
        &self.effects
    }
    fn select(
        &mut self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<super::iteration_program::IterationInput, ExecutionError> {
        let (object, snapshot) = &self.matching[index];
        let it = TagKey::from("__it__");
        if let Some(shuffle) = &self.shuffle {
            let owner = ctx.with_object_tag(it.clone(), vec![snapshot.clone()], |ctx| {
                resolve_player_filter(game, &shuffle.player, ctx)
            })?;
            if !self.owners.contains(&owner) {
                self.owners.push(owner);
            }
        }
        Ok(super::iteration_program::IterationInput {
            object: Some(*object),
            player: snapshot.controller,
            tags: vec![(it, vec![snapshot.clone()])],
        })
    }
    fn postlude(&mut self) -> Vec<Effect> {
        self.owners
            .iter()
            .map(|owner| {
                crate::effects::cards::shuffle_library_action(
                    *owner,
                    &[],
                    1,
                    "library shuffled after iterated movement",
                )
            })
            .collect()
    }
}
fn object_iteration_cursor(
    effect: &ForEachObject,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Box<dyn crate::effects::ActionProgramCursor> {
    let matching = matching_objects(effect, game, ctx);
    let it_tag = TagKey::from("__it__");
    let batched_effects = if let [child] = effect.effects.as_slice()
        && let Some(sequence) = child.downcast_ref::<crate::effects::SequenceEffect>()
    {
        sequence.effects.as_slice()
    } else {
        effect.effects.as_slice()
    };
    let (effects, shuffle) = if let [move_effect, shuffle_effect] = batched_effects
        && let Some(movement) = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && matches!(movement.target.base(), ChooseSpec::Iterated)
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && matches!(&shuffle.player, crate::target::PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(tag)) if tag == &it_tag)
    {
        (vec![move_effect.clone()], Some(shuffle.clone()))
    } else {
        (effect.effects.clone(), None)
    };
    super::iteration_program::selected_iteration_cursor(
        Box::new(ObjectIterationPlan {
            matching,
            effects,
            shuffle,
            owners: Vec::new(),
        }),
        ctx,
    )
}

impl EffectExecutor for ForEachObject {
    fn supports_prepared_action_program(&self) -> bool {
        self.effects
            .iter()
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if self.supports_damage_action_cohort() && self.supports_simultaneous_player_action() {
            let proposal = self.prepare_simultaneous_player_action(game, ctx)?;
            return Ok(Some(super::action_program::prepared_damage_program_cursor(
                Effect::new(self.clone()),
                proposal,
            )));
        }
        Ok(Some(object_iteration_cursor(self, game, ctx)))
    }

    fn supports_damage_action_cohort(&self) -> bool {
        matches!(self.effects.as_slice(), [child] if
            child.0.shares_iterated_damage_action() && child.0.supports_damage_action_cohort())
    }
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn own_preflight_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![ChooseSpec::All(self.filter.clone())]
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.supports_damage_action_cohort()
            && self.effects[0].0.supports_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self, game: &GameState, ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        if !self.supports_simultaneous_player_action() {
            return Err(ExecutionError::Impossible(
                "object iteration requires selected-program scheduling".into(),
            ));
        }

        let matching = matching_objects(self, game, ctx);
        let it_tag = TagKey::from("__it__");
        super::with_iteration_tags(ctx, vec![(it_tag.clone(), None)], |ctx| {
            let mut iterations = Vec::with_capacity(matching.len());
            for (object_id, snapshot) in &matching {
                ctx.set_tagged_objects(it_tag.clone(), vec![snapshot.clone()]);
                let proposals = super::with_object_iteration(
                    ctx,
                    *object_id,
                    snapshot.controller,
                    Vec::new(),
                    |ctx| {
                        self.effects
                            .iter()
                            .map_while(|effect| {
                                if ctx.decision_maker.awaiting_choice() {
                                    return None;
                                }
                                Some(effect.prepare_simultaneous_player_action(game, ctx).map(
                                    |inner| {
                                        super::scope_prepared_iteration(
                                            inner,
                                            *object_id,
                                            snapshot.controller,
                                            vec![(it_tag.clone(), vec![snapshot.clone()])],
                                        )
                                    },
                                ))
                            })
                            .collect::<Result<Vec<_>, ExecutionError>>()
                    },
                )?;
                iterations.push(proposals);
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
            Ok(Box::new(ForEachObjectProposal { iterations })
                as Box<dyn SimultaneousEffectProposal>)
        })
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
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_object_iterations(self, game, ctx),
        )
    }

    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects.iter().all(crate::effects::replacement::replacement_effect_supported)
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

}

fn execute_object_iterations(
    effect: &ForEachObject,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if !effect.effects.is_empty()
        && effect
            .effects
            .iter()
            .all(|child| child.0.shares_iterated_damage_action())
        && effect.supports_damage_action_cohort()
    {
        let proposal = effect.prepare_simultaneous_player_action(game, ctx)?;
        let cursor = super::action_program::prepared_damage_program_cursor(
            Effect::new(effect.clone()),
            proposal,
        );
        return super::action_program::execute_action_program_with_outputs(
            cursor,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        );
    }
    let cursor = object_iteration_cursor(effect, game, ctx);
    // A compound body contains distinct authored actions. Only a direct
    // shared-damage child may keep one simultaneous instruction open.
    let opened = effect.effects.len() <= 1
        && effect
            .effects
            .iter()
            .all(|child| child.0.shares_iterated_damage_action())
        && game.open_simultaneous_action();
    let result = super::action_program::execute_action_program_with_outputs(
        cursor,
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    );
    game.close_simultaneous_action(opened);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::CounterEffect;
    use crate::events::{DamageEvent, DamageTarget};
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::{ChooseSpec, ObjectRef, PlayerFilter, TaggedObjectConstraint};
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_creature_with_stats(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let id = game.new_object_id();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    fn create_library_card(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, controller, Zone::Library);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_for_each_no_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // No creatures on battlefield
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Empty aggregate returns Resolved (no effects executed)
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_for_each_multiple_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 creatures
        create_creature(&mut game, "Bear 1", alice);
        create_creature(&mut game, "Bear 2", alice);
        create_creature(&mut game, "Bear 3", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        // Gained 1 life for each creature (3 total)
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_for_each_filtered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create 2 creatures for Alice, 1 for Bob
        create_creature(&mut game, "Alice Bear 1", alice);
        create_creature(&mut game, "Alice Bear 2", alice);
        create_creature(&mut game, "Bob Bear", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // Only count creatures Alice controls
        let effect = ForEachObject::new(
            ObjectFilter::creature().you_control(),
            vec![Effect::gain_life(1)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn each_object_power_damage_gameplay_uses_each_source_and_its_own_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let ability_source = game.new_object_id();
        let below_threshold = create_creature_with_stats(&mut game, "Small Source", alice, 3, 3);
        let four_power = create_creature_with_stats(&mut game, "Four Source", alice, 4, 4);
        let six_power = create_creature_with_stats(&mut game, "Six Source", alice, 6, 6);
        let opposing_source = create_creature_with_stats(&mut game, "Opposing Source", bob, 8, 8);
        let target = create_creature_with_stats(&mut game, "Chosen Target", bob, 0, 20);

        let target_tag = TagKey::from("targeted_0");
        let target_snapshot =
            ObjectSnapshot::from_object(game.object(target).expect("target exists"), &game);
        let mut ctx = ExecutionContext::new_default(ability_source, alice);
        ctx.tag_object(target_tag.clone(), target_snapshot);

        let source_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::You)
            .with_power(crate::filter::Comparison::GreaterThanOrEqual(4));
        let target_filter = ObjectFilter::permanent()
            .match_tagged(target_tag, TaggedOpbjectRelation::IsTaggedObject);
        let effect = ForEachObject::new(
            source_filter,
            vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Iterated,
                Effect::deal_damage(
                    crate::effect::Value::PowerOf(Box::new(ChooseSpec::Iterated)),
                    ChooseSpec::Object(target_filter),
                ),
            ))],
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("source loop resolves");
        let damage = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .map(|event| (event.source, event.amount, event.target))
            .collect::<Vec<_>>();

        assert_eq!(game.damage_on(target), 10);
        assert_eq!(damage.len(), 2, "{damage:?}");
        assert!(damage.iter().any(|(source, amount, damage_target)| {
            *source == four_power
                && *amount == 4
                && matches!(damage_target, crate::events::DamageTarget::Object(id) if *id == target)
        }));
        assert!(damage.iter().any(|(source, amount, damage_target)| {
            *source == six_power
                && *amount == 6
                && matches!(damage_target, crate::events::DamageTarget::Object(id) if *id == target)
        }));
        assert!(
            !damage
                .iter()
                .any(|(source, _, _)| { *source == below_threshold || *source == opposing_source })
        );
    }

    #[test]
    fn each_selected_creature_deals_its_toughness_to_the_other_selected_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_source = game.new_object_id();
        let your_creature = create_creature_with_stats(&mut game, "Your Creature", alice, 1, 5);
        let opposing_creature =
            create_creature_with_stats(&mut game, "Opposing Creature", bob, 1, 3);
        let mut ctx = ExecutionContext::new_default(spell_source, alice).with_targets(vec![
            crate::ResolvedTarget::Object(your_creature),
            crate::ResolvedTarget::Object(opposing_creature),
        ]);
        let pair_tag = crate::tag::TagKey::from("chosen_pair");
        ctx.tag_object(
            pair_tag.clone(),
            crate::snapshot::ObjectSnapshot::from_object(
                game.object(your_creature).expect("your creature exists"),
                &game,
            ),
        );
        ctx.tag_object(
            pair_tag.clone(),
            crate::snapshot::ObjectSnapshot::from_object(
                game.object(opposing_creature)
                    .expect("opposing creature exists"),
                &game,
            ),
        );
        let mut pair_filter = ObjectFilter::creature().match_tagged(
            pair_tag,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        pair_filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Those));
        let effect = ForEachObject::new(
            pair_filter.clone(),
            vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Iterated,
                Effect::deal_damage(
                    crate::effect::Value::ToughnessOf(Box::new(ChooseSpec::Iterated)),
                    ChooseSpec::Object(pair_filter.other()),
                ),
            ))],
        );
        assert!(
            effect.effects[0].0.get_target_spec().is_none(),
            "the other member of an already targeted pair must not announce a third target"
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("reciprocal toughness damage resolves");
        let damage = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .map(|event| (event.source, event.amount, event.target))
            .collect::<Vec<_>>();

        assert_eq!(game.damage_on(your_creature), 3);
        assert_eq!(game.damage_on(opposing_creature), 5);
        assert!(damage.iter().any(|(source, amount, target)| {
            *source == your_creature
                && *amount == 5
                && matches!(target, DamageTarget::Object(id) if *id == opposing_creature)
        }));
        assert!(damage.iter().any(|(source, amount, target)| {
            *source == opposing_creature
                && *amount == 3
                && matches!(target, DamageTarget::Object(id) if *id == your_creature)
        }));
    }

    #[test]
    fn test_for_each_clone_box() {
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ForEachObject"));
    }

    #[test]
    fn test_for_each_sets_iterated_object_for_inner_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let c1 = create_creature(&mut game, "Bear 1", alice);
        let c2 = create_creature(&mut game, "Bear 2", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForEachObject::new(
            ObjectFilter::creature().you_control(),
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let c1_obj = game.object(c1).expect("c1 should exist");
        let c2_obj = game.object(c2).expect("c2 should exist");
        assert_eq!(c1_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
        assert_eq!(c2_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
    }

    #[test]
    fn only_simultaneous_damage_player_fanout_shares_the_outer_object_batch() {
        let damage = Effect::deal_damage(1, ChooseSpec::Player(PlayerFilter::IteratedPlayer));
        let mut players = crate::effects::ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![damage],
        );
        assert!(Effect::new(players.clone()).0.shares_iterated_damage_action());
        players.sequential = true;
        assert!(!Effect::new(players.clone()).0.shares_iterated_damage_action());
        players.sequential = false;
        players.stop_after_first_happened = true;
        assert!(!Effect::new(players.clone()).0.shares_iterated_damage_action());
        players.stop_after_first_happened = false;
        players.effects.push(Effect::new(crate::effects::DrawCardsEffect::you(1)));
        assert!(!Effect::new(players).0.shares_iterated_damage_action());
    }

    #[test]
    fn simultaneous_player_action_can_propose_nested_controlled_object_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_creature = create_creature(&mut game, "Alice Bear", alice);
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let mut controlled_creature = ObjectFilter::creature();
        controlled_creature.controller = Some(PlayerFilter::IteratedPlayer);
        let nested = Effect::new(ForEachObject::new(
            controlled_creature,
            vec![Effect::deal_damage(
                crate::effect::Value::Fixed(1),
                ChooseSpec::Iterated,
            )],
        ));
        let effect = crate::effects::ForPlayersEffect::new(PlayerFilter::Any, vec![nested]);

        effect
            .execute(&mut game, &mut ctx)
            .expect("nested object fanout should participate in simultaneous proposals");

        assert_eq!(game.damage_on(alice_creature), 1);
        assert_eq!(game.damage_on(bob_creature), 1);
    }

    #[test]
    fn sequence_wrapped_move_then_shuffle_batches_each_owner_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_creature(&mut game, "Bear 1", alice);
        create_creature(&mut game, "Bear 2", alice);
        create_creature(&mut game, "Bear 3", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let sequence = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
            Effect::move_to_zone(ChooseSpec::Iterated, Zone::Library, true),
            Effect::shuffle_library_player(PlayerFilter::OwnerOf(ObjectRef::tagged("__it__"))),
        ]));
        let effect = ForEachObject::new(ObjectFilter::creature().you_control(), vec![sequence]);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("the move-and-shuffle loop resolves");

        assert_eq!(
            game.zone_ids(Zone::Library).count(),
            3,
            "all iterated creatures move before the library is shuffled"
        );
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| event.downcast::<ShuffleLibraryEvent>().is_some())
                .count(),
            1,
            "one owner must not shuffle once per iterated object"
        );
    }

    #[test]
    fn test_for_each_uses_tagged_nonbattlefield_candidates_for_is_tagged_filters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let revealed_creature = create_library_card(
            &mut game,
            "Revealed Creature",
            alice,
            vec![CardType::Creature],
        );
        let revealed_land =
            create_library_card(&mut game, "Revealed Land", alice, vec![CardType::Land]);

        ctx.tag_object(
            "revealed_0",
            ObjectSnapshot::from_object(game.object(revealed_creature).unwrap(), &game),
        );
        ctx.tag_object(
            "revealed_0",
            ObjectSnapshot::from_object(game.object(revealed_land).unwrap(), &game),
        );

        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("revealed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let initial_life = game.player(alice).unwrap().life;
        let effect = ForEachObject::new(filter, vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 1);
    }

    #[test]
    fn tagged_for_each_uses_lki_after_the_producing_action_changes_zones() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let destroyed = create_creature(&mut game, "Destroyed Creature", bob);
        let destroyed_snapshot =
            ObjectSnapshot::from_object(game.object(destroyed).expect("creature exists"), &game);
        let destroyed_tag = TagKey::from("destroyed_0");

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object(destroyed_tag.clone(), destroyed_snapshot);
        game.move_object(
            destroyed,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
        )
        .expect("the tagged permanent moves to its graveyard");

        let filter = ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .match_tagged(destroyed_tag, TaggedOpbjectRelation::IsTaggedObject);
        let initial_bob_life = game.player(bob).expect("Bob exists").life;
        let effect = ForEachObject::new(
            filter,
            vec![Effect::gain_life_player(
                1,
                ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::tagged("__it__"))),
            )],
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("the LKI-backed tagged loop resolves");

        assert_eq!(result.as_count(), Some(1));
        assert_eq!(
            game.player(bob).expect("Bob exists").life,
            initial_bob_life + 1,
            "the follow-up must use the tagged permanent's last-known controller"
        );
    }

    #[test]
    fn whirlwind_denial_style_loop_visits_opponent_stack_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let opponent_spell_card = CardBuilder::new(CardId::new(), "Opponent Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let opponent_spell = game.create_object_from_card(&opponent_spell_card, bob, Zone::Stack);
        game.push_to_stack(StackEntry::new(opponent_spell, bob));

        let ability_source = create_creature(&mut game, "Opponent Ability Source", bob);
        game.push_to_stack(StackEntry::ability(
            ability_source,
            bob,
            vec![Effect::draw(1)],
        ));

        let your_spell_card = CardBuilder::new(CardId::new(), "Your Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let your_spell = game.create_object_from_card(&your_spell_card, alice, Zone::Stack);
        game.push_to_stack(StackEntry::new(your_spell, alice));

        let mut stack_filter = ObjectFilter::default();
        stack_filter.zone = Some(Zone::Stack);
        stack_filter.controller = Some(PlayerFilter::Opponent);
        let effect = ForEachObject::new(
            stack_filter,
            vec![Effect::new(CounterEffect::new(ChooseSpec::Iterated))],
        );
        let mut ctx = ExecutionContext::new_default(your_spell, alice);

        effect
            .execute(&mut game, &mut ctx)
            .expect("stack-zone iteration should resolve");

        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == opponent_spell),
            "the opponent's spell should be countered"
        );
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == ability_source),
            "the opponent's ability should be countered"
        );
        assert!(
            game.stack.iter().any(|entry| entry.object_id == your_spell),
            "your stack object should remain"
        );
        assert_eq!(
            game.object(ability_source)
                .expect("the ability source should remain on the battlefield")
                .zone,
            Zone::Battlefield
        );
    }
}

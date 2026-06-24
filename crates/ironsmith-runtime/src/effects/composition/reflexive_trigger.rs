//! Reflexive trigger effect implementation.

use std::collections::HashMap;

use crate::card::LinkedFaceLayout;
use crate::decisions::context::{TargetRequirementContext, TargetsContext};
use crate::effect::{
    Effect, EffectId, EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt,
    OutcomeObjectMemory,
};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::object::ObjectKind;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::targeting::normalize_targets_for_requirements;

/// Effect that creates a reflexive triggered ability from a prior effect result.
///
/// This models clauses like "When you do, ..." where the follow-up trigger is
/// created only if an earlier effect satisfied a result predicate, and targets
/// are chosen when that new ability is put onto the stack.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveTriggerEffect {
    /// The prior effect result to inspect.
    pub condition: EffectId,
    /// How to evaluate the prior effect result.
    pub predicate: EffectPredicate,
    /// Effects for the reflexive triggered ability.
    pub effects: Vec<Effect>,
    /// Target choices that must be made when the reflexive ability is created.
    pub choices: Vec<ChooseSpec>,
}

impl ReflexiveTriggerEffect {
    pub fn new(
        condition: EffectId,
        predicate: EffectPredicate,
        effects: Vec<Effect>,
        choices: Vec<ChooseSpec>,
    ) -> Self {
        Self {
            condition,
            predicate,
            effects,
            choices,
        }
    }
}

fn describe_choice(spec: &ChooseSpec) -> String {
    match spec.base() {
        ChooseSpec::Player(_) => "target player".to_string(),
        ChooseSpec::Object(_) => "target object".to_string(),
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => "target".to_string(),
        ChooseSpec::PlayerOrPlaneswalker(_) => "target player or planeswalker".to_string(),
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "target attacked player or planeswalker".to_string()
        }
        _ => "target".to_string(),
    }
}

fn choose_reflexive_targets(
    game: &GameState,
    ctx: &mut ExecutionContext,
    choices: &[ChooseSpec],
) -> Option<Vec<crate::game_state::Target>> {
    let mut chosen_targets = Vec::new();

    for spec in choices {
        let count = spec.count();
        let legal_targets = crate::targeting::compute_legal_targets_with_tagged_objects(
            game,
            spec,
            ctx.controller,
            Some(ctx.source),
            Some(&ctx.tagged_objects),
        );

        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, spec, &legal_targets);
        if !crate::targeting::has_enough_legal_targets_for_spec(
            game,
            spec,
            &legal_targets,
            count.min,
        ) {
            return None;
        }

        let targets_ctx = TargetsContext::new(
            ctx.controller,
            ctx.source,
            "reflexive triggered ability",
            vec![TargetRequirementContext {
                description: describe_choice(spec),
                legal_targets: legal_targets.clone(),
                legal_target_sets,
                min_targets: count.min,
                max_targets: count.max,
            }],
        );
        let selected = ctx.decision_maker.decide_targets(game, &targets_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        let selected = normalize_targets_for_requirements(&targets_ctx.requirements, selected)?;

        chosen_targets.extend(selected);
    }

    Some(chosen_targets)
}

fn snapshot_from_memory(game: &GameState, memory: &OutcomeObjectMemory) -> ObjectSnapshot {
    let mut snapshot = game
        .object(memory.object_id)
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .unwrap_or_else(|| ObjectSnapshot {
            object_id: memory.object_id,
            stable_id: memory.stable_id,
            kind: if memory.is_token {
                ObjectKind::Token
            } else {
                ObjectKind::Card
            },
            card: None,
            controller: memory.controller,
            owner: memory.owner,
            name: String::new(),
            mana_cost: None,
            colors: memory.colors,
            supertypes: Vec::new(),
            card_types: memory.card_types.clone(),
            subtypes: memory.subtypes.clone(),
            compiled_card_text: String::new(),
            other_face: None,
            other_face_name: None,
            linked_face_layout: LinkedFaceLayout::None,
            power: memory.power,
            toughness: memory.toughness,
            base_power: memory.power,
            base_toughness: memory.toughness,
            loyalty: None,
            defense: None,
            abilities: Vec::new(),
            aura_attach_filter: None,
            x_value: None,
            cast_order_this_turn: None,
            mana_spent_to_cast: crate::player::ManaPool::default(),
            counters: HashMap::new(),
            is_token: memory.is_token,
            tapped: false,
            attacking: false,
            flipped: false,
            face_down: false,
            transform_count: 0,
            attached_to: None,
            attachments: Vec::new(),
            was_enchanted: false,
            is_monstrous: false,
            is_commander: false,
            zone: memory.zone,
        });

    snapshot.stable_id = memory.stable_id;
    snapshot.controller = memory.controller;
    snapshot.owner = memory.owner;
    snapshot.zone = memory.zone;
    snapshot.power = memory.power;
    snapshot.toughness = memory.toughness;
    snapshot.card_types = memory.card_types.clone();
    snapshot.colors = memory.colors;
    snapshot.subtypes = memory.subtypes.clone();
    snapshot.is_token = memory.is_token;
    snapshot
}

fn snapshots_from_object_ids(
    game: &GameState,
    ids: &[crate::ids::ObjectId],
) -> Vec<ObjectSnapshot> {
    ids.iter()
        .filter_map(|id| game.object(*id))
        .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game))
        .collect()
}

fn reflexive_it_snapshots(game: &GameState, outcome: &EffectOutcome) -> Vec<ObjectSnapshot> {
    if let Some(memory) = outcome.affected_object_memory()
        && !memory.is_empty()
    {
        return memory
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect();
    }
    if let Some(memory) = outcome.chosen_object_memory()
        && !memory.is_empty()
    {
        return memory
            .iter()
            .map(|memory| snapshot_from_memory(game, memory))
            .collect();
    }
    if let Some(ids) = outcome.affected_objects()
        && !ids.is_empty()
    {
        return snapshots_from_object_ids(game, ids);
    }
    if let Some(ids) = outcome.chosen_objects()
        && !ids.is_empty()
    {
        return snapshots_from_object_ids(game, ids);
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::{ReflexiveTriggerEffect, choose_reflexive_targets};
    use crate::cards::definitions::{grizzly_bears, lightning_bolt};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::TargetsContext;
    use crate::effect::{
        ChoiceCount, Effect, EffectId, EffectOutcome, EffectPredicate, OutcomeObjectMemory,
    };
    use crate::effects::{EffectExecutor, ExecutionContext};
    use crate::game_state::{GameState, Target};
    use crate::ids::PlayerId;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ChooseSpec;
    use crate::zone::Zone;

    struct DuplicateTargetDecisionMaker {
        target: Target,
    }

    impl DecisionMaker for DuplicateTargetDecisionMaker {
        fn decide_targets(&mut self, _game: &GameState, _ctx: &TargetsContext) -> Vec<Target> {
            vec![self.target, self.target]
        }
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn reflexive_targets_are_normalized_per_requirement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&lightning_bolt(), alice, Zone::Stack);
        let first = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(first),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let choices =
            vec![ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2))];

        let selected = choose_reflexive_targets(&game, &mut ctx, &choices).expect("valid targets");

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], Target::Object(first));
        assert_eq!(selected[1], Target::Object(second));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn reflexive_trigger_pushes_stack_ability_with_captured_context() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source =
            game.create_object_from_definition(&lightning_bolt(), alice, Zone::Battlefield);
        let tagged = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let tagged_snapshot =
            ObjectSnapshot::from_object(game.object(tagged).expect("tagged object"), &game);

        let condition = EffectId(77);
        let reflexive = ReflexiveTriggerEffect::new(
            condition,
            EffectPredicate::Happened,
            vec![Effect::draw(1)],
            Vec::new(),
        );

        let mut dm = DuplicateTargetDecisionMaker {
            target: Target::Object(tagged),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.store_outcome(
            condition,
            EffectOutcome::count(1).with_affected_object_memory(vec![
                OutcomeObjectMemory::from_snapshot(&tagged_snapshot),
            ]),
        );
        ctx.x_value = Some(7);
        ctx.combat.defending_player = Some(bob);
        ctx.set_tagged_objects(TagKey::from("sacrificed"), vec![tagged_snapshot.clone()]);

        reflexive
            .execute(&mut game, &mut ctx)
            .expect("reflexive trigger should push a stack ability");

        let entry = game.stack.last().expect("reflexive ability on stack");
        assert!(entry.is_ability);
        assert_eq!(entry.object_id, source);
        assert_eq!(entry.controller, alice);
        assert_eq!(entry.x_value, Some(7));
        assert_eq!(entry.defending_player, Some(bob));
        let captured = entry
            .tagged_objects
            .get(&TagKey::from("sacrificed"))
            .expect("tagged object snapshots carried to stack entry");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].object_id, tagged_snapshot.object_id);
        let it = entry
            .tagged_objects
            .get(&TagKey::from("__it__"))
            .expect("condition object memory exposed as reflexive it tag");
        assert_eq!(it.len(), 1);
        assert_eq!(it[0].object_id, tagged_snapshot.object_id);
        assert_eq!(
            entry.source_name.as_deref(),
            Some(game.object(source).expect("source object").name.as_str())
        );
        assert!(entry.source_snapshot.is_some());
    }
}

impl EffectExecutor for ReflexiveTriggerEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = ctx
            .get_outcome(self.condition)
            .cloned()
            .ok_or(ExecutionError::EffectNotFound(self.condition))?;
        if !self.predicate.evaluate_outcome(&outcome) {
            return Ok(EffectOutcome::resolved());
        }
        let fallback_it_snapshots = reflexive_it_snapshots(game, &outcome);

        let targets = choose_reflexive_targets(game, ctx, &self.choices)
            .ok_or(ExecutionError::InvalidTarget)?;

        let mut tagged_objects = ctx.tagged_objects.clone();
        let it_tag = TagKey::from("__it__");
        if !tagged_objects.contains_key(&it_tag) && !fallback_it_snapshots.is_empty() {
            tagged_objects.insert(it_tag, fallback_it_snapshots);
        }

        let mut entry = StackEntry::ability(ctx.source, ctx.controller, self.effects.clone())
            .with_targets(targets)
            .with_optional_costs_paid(ctx.optional_costs_paid.clone())
            .with_tagged_objects(tagged_objects);

        if let Some(x) = ctx.x_value {
            entry = entry.with_x(x);
        }
        if let Some(defending_player) = ctx.combat.defending_player {
            entry = entry.with_defending_player(defending_player);
        }
        if let Some(source) = game.object(ctx.source) {
            entry = entry.with_source_info(source.stable_id, source.name.clone());
        } else if let Some(snapshot) = ctx.source_snapshot.clone() {
            entry = entry
                .with_source_info(snapshot.stable_id, snapshot.name.clone())
                .with_source_snapshot(snapshot);
        }

        game.push_to_stack(entry);
        Ok(EffectOutcome::count(1))
    }
}

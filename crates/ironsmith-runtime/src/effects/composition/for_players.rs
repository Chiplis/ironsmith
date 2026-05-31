//! ForPlayers effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::filter::PlayerFilterExt;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use std::collections::HashMap;

/// Effect that applies effects once for each player matching a filter.
///
/// Sets `ctx.iterated_player` for each iteration, allowing inner effects
/// to reference the current player via `PlayerFilter::IteratedPlayer`.
///
/// # Fields
///
/// * `filter` - Filter for which players to iterate over
/// * `effects` - Effects to execute for each matching player
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to each opponent
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Opponent,
///     vec![Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer))],
/// );
///
/// // Each player draws a card
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Any,
///     vec![Effect::target_draws(1, PlayerFilter::IteratedPlayer)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect {
    /// Filter for which players to iterate over.
    pub filter: PlayerFilter,
    /// Effects to execute for each matching player.
    pub effects: Vec<Effect>,
}

impl ForPlayersEffect {
    /// Create a new ForPlayers effect.
    pub fn new(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self { filter, effects }
    }
}

impl EffectExecutor for ForPlayersEffect {
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
        let filter_ctx = ctx.filter_context(game);

        // Iterate over all players that match the filter
        let players: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .filter(|p| self.filter.matches_player(p.id, &filter_ctx))
            .map(|p| p.id)
            .collect();

        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();
        let mut player_counts = Vec::new();
        let mut player_affected_memory = Vec::new();
        let base_tagged_objects = ctx.tagged_objects.clone();
        let mut accumulated_tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>> = HashMap::new();

        for player_id in players {
            ctx.tagged_objects = base_tagged_objects.clone();
            let iteration_result = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                let start = outcomes.len();
                // Execute all inner effects for this player
                for effect in &self.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
                let count =
                    EffectOutcome::aggregate_summing_counts(outcomes[start..].iter().cloned())
                        .as_count()
                        .unwrap_or(0);
                player_counts.push((player_id, count));
                let iteration_outcome =
                    EffectOutcome::aggregate_summing_counts(outcomes[start..].iter().cloned());
                if let Some(memory) = iteration_outcome.affected_object_memory()
                    && !memory.is_empty()
                {
                    player_affected_memory.push((player_id, memory.to_vec()));
                }
                Ok::<(), ExecutionError>(())
            });

            accumulate_iteration_tags(
                &base_tagged_objects,
                &ctx.tagged_objects,
                &mut accumulated_tagged_objects,
            );
            ctx.tagged_objects = base_tagged_objects.clone();
            iteration_result?;
        }

        ctx.tagged_objects = base_tagged_objects;
        for (tag, snapshots) in accumulated_tagged_objects {
            ctx.tag_objects_unique(tag, snapshots);
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes)
            .with_player_counts(player_counts)
            .with_player_affected_object_memory(player_affected_memory))
    }
}

fn accumulate_iteration_tags(
    base: &HashMap<TagKey, Vec<ObjectSnapshot>>,
    current: &HashMap<TagKey, Vec<ObjectSnapshot>>,
    accumulated: &mut HashMap<TagKey, Vec<ObjectSnapshot>>,
) {
    for (tag, snapshots) in current {
        if base
            .get(tag)
            .is_some_and(|base_snapshots| same_snapshots(base_snapshots, snapshots))
        {
            continue;
        }
        let entry = accumulated.entry(tag.clone()).or_default();
        for snapshot in snapshots {
            let existed_before = base
                .get(tag)
                .is_some_and(|base_snapshots| contains_snapshot(base_snapshots, snapshot));
            let already_accumulated = contains_snapshot(entry, snapshot);
            if !existed_before && !already_accumulated {
                entry.push(snapshot.clone());
            }
        }
    }
}

fn same_snapshots(left: &[ObjectSnapshot], right: &[ObjectSnapshot]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| {
                left.object_id == right.object_id && left.stable_id == right.stable_id
            })
}

fn contains_snapshot(snapshots: &[ObjectSnapshot], needle: &ObjectSnapshot) -> bool {
    snapshots.iter().any(|snapshot| {
        snapshot.object_id == needle.object_id && snapshot.stable_id == needle.stable_id
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn for_players_sums_count_results_across_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).expect("alice").life, 19);
        assert_eq!(game.player(PlayerId::from_index(1)).expect("bob").life, 19);
    }

    #[test]
    fn for_players_records_per_player_count_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(
                crate::effect::Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(
            result.player_counts(),
            Some([(alice, 1), (bob, 1)].as_slice())
        );
    }

    #[test]
    fn for_players_records_per_player_affected_object_memory_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let alice_card = game.new_object_id();
        let bob_card = game.new_object_id();
        let alice_memory = crate::effect::OutcomeObjectMemory {
            object_id: alice_card,
            stable_id: crate::ids::StableId::from(alice_card),
            controller: alice,
            owner: alice,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 1,
            card_types: vec![crate::types::CardType::Creature],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
        };
        let bob_memory = crate::effect::OutcomeObjectMemory {
            object_id: bob_card,
            stable_id: crate::ids::StableId::from(bob_card),
            controller: bob,
            owner: bob,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 2,
            card_types: vec![crate::types::CardType::Instant],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
        };

        let result = EffectOutcome::aggregate_summing_counts(vec![
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![alice_memory.clone()])
                .with_player_affected_object_memory(vec![(alice, vec![alice_memory])]),
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![bob_memory.clone()])
                .with_player_affected_object_memory(vec![(bob, vec![bob_memory])]),
        ]);

        let partitions = result
            .player_affected_object_memory()
            .expect("per-player affected memory");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, alice);
        assert_eq!(partitions[0].1[0].controller, alice);
        assert_eq!(partitions[1].0, bob);
        assert_eq!(partitions[1].1[0].controller, bob);

        let effect = ForPlayersEffect::new(PlayerFilter::Any, Vec::new());
        let empty_result = effect
            .execute(&mut game, &mut ctx)
            .expect("empty per-player effect should resolve");
        assert!(empty_result.player_affected_object_memory().is_none());
    }

    #[test]
    fn for_each_opponent_reveal_keeps_each_opponents_revealed_card_partitioned() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let bob_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1001), "Bob Top")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let cara_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1002), "Cara Top")
                .card_types(vec![crate::types::CardType::Instant])
                .build();
        let bob_id = game.create_object_from_card(&bob_card, bob, crate::zone::Zone::Library);
        let cara_id = game.create_object_from_card(&cara_card, cara, crate::zone::Zone::Library);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::reveal_top_cards(
                PlayerFilter::IteratedPlayer,
                crate::effect::Value::Fixed(1),
                crate::tag::TagKey::from("revealed"),
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("for each opponent reveal should resolve");

        assert_eq!(result.events.len(), 2);
        assert_eq!(
            result.affected_object_memory().map(|memory| memory.len()),
            Some(2)
        );
        let partitions = result
            .player_affected_object_memory()
            .expect("per-player reveal partitions");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, bob);
        assert_eq!(partitions[0].1.len(), 1);
        assert_eq!(partitions[0].1[0].object_id, bob_id);
        assert_eq!(partitions[1].0, cara);
        assert_eq!(partitions[1].1.len(), 1);
        assert_eq!(partitions[1].1[0].object_id, cara_id);
    }
}

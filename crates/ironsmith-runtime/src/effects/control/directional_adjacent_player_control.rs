//! Directional adjacent-player control choice implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::ControlChangedEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type DirectionalAdjacentPlayerControlEffect =
    ironsmith_core::DirectionalAdjacentPlayerControlEffect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeatingDirection {
    Left,
    Right,
}

fn chosen_direction(
    game: &GameState,
    source: ObjectId,
    effect: &DirectionalAdjacentPlayerControlEffect,
) -> Result<SeatingDirection, ExecutionError> {
    let option = game.chosen_named_option(source).ok_or_else(|| {
        ExecutionError::UnresolvableValue("directional choice was not made".to_string())
    })?;
    if option.eq_ignore_ascii_case(&effect.left_option) {
        Ok(SeatingDirection::Left)
    } else if option.eq_ignore_ascii_case(&effect.right_option) {
        Ok(SeatingDirection::Right)
    } else {
        Err(ExecutionError::UnresolvableValue(format!(
            "unsupported directional choice '{option}'"
        )))
    }
}

fn players_in_direction(
    game: &GameState,
    start: PlayerId,
    direction: SeatingDirection,
) -> Vec<PlayerId> {
    let players = game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .map(|player| player.id)
        .collect::<Vec<_>>();
    let Some(start_idx) = players.iter().position(|player| *player == start) else {
        return Vec::new();
    };
    let len = players.len();
    (0..len)
        .map(|offset| match direction {
            SeatingDirection::Left => players[(start_idx + offset) % len],
            SeatingDirection::Right => players[(start_idx + len - (offset % len)) % len],
        })
        .collect()
}

fn adjacent_player(ordered_players: &[PlayerId], player: PlayerId) -> Option<PlayerId> {
    let len = ordered_players.len();
    if len == 0 {
        return None;
    }
    let idx = ordered_players
        .iter()
        .position(|candidate| *candidate == player)?;
    Some(ordered_players[(idx + 1) % len])
}

fn matching_adjacent_objects(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::filter::ObjectFilter,
    adjacent: PlayerId,
) -> Vec<ObjectId> {
    let filter_ctx = ctx.filter_context(game);
    game.battlefield
        .iter()
        .filter_map(|id| game.object(*id).map(|object| (*id, object)))
        .filter(|(_, object)| {
            object.zone == Zone::Battlefield
                && game.controller_of(object) == adjacent
                && filter.matches(object, &filter_ctx, game)
        })
        .map(|(id, _)| id)
        .collect()
}

impl EffectExecutor for DirectionalAdjacentPlayerControlEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let direction = chosen_direction(game, ctx.source, self)?;
        let ordered_players = players_in_direction(game, ctx.controller, direction);
        if ordered_players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut chosen = Vec::new();
        for chooser in &ordered_players {
            let Some(adjacent) = adjacent_player(&ordered_players, *chooser) else {
                continue;
            };
            let candidates = matching_adjacent_objects(game, ctx, &self.filter, adjacent);
            if candidates.is_empty() {
                continue;
            }
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose a controlled object in the chosen direction".to_string(),
                candidates.clone(),
                1,
                Some(1),
            );
            let mut selected =
                make_decision(game, ctx.decision_maker, *chooser, Some(ctx.source), spec);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let selected = selected
                .pop()
                .filter(|object_id| candidates.contains(object_id))
                .unwrap_or(candidates[0]);
            chosen.push((*chooser, selected));
        }

        let mut outcomes = Vec::new();
        for (new_controller, object_id) in chosen {
            let previous_controller = game.current_controller(object_id);
            let apply = ApplyContinuousEffect::new(
                EffectTarget::Specific(object_id),
                Modification::ChangeController(new_controller),
                Until::Forever,
            );
            let mut outcome = execute_effect(game, &Effect::new(apply), ctx)?;
            if let Some(previous_controller) = previous_controller
                && previous_controller != new_controller
            {
                game.clear_soulbond_pair(object_id);
                outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                    ControlChangedEvent::new(object_id, previous_controller, new_controller),
                    ctx.provenance,
                ));
            }
            outcomes.push(outcome);
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{SelectObjectsContext, SelectOptionsContext};
    use crate::ids::CardId;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct NamedObjectDecisionMaker {
        object_matches: VecDeque<String>,
    }

    impl NamedObjectDecisionMaker {
        fn new(matches: &[&str]) -> Self {
            Self {
                object_matches: matches
                    .iter()
                    .map(|value| value.to_ascii_lowercase())
                    .collect(),
            }
        }
    }

    impl DecisionMaker for NamedObjectDecisionMaker {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }

        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if let Some(needle) = self.object_matches.pop_front()
                && let Some(candidate) = ctx.candidates.iter().find(|candidate| {
                    candidate.legal
                        && game.object(candidate.id).is_some_and(|object| {
                            object.name.to_ascii_lowercase().contains(&needle)
                        })
                })
            {
                return vec![candidate.id];
            }
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn directional_control_uses_left_adjacent_players() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.new_object_id();
        game.set_chosen_named_option(source, "left".to_string());
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let cara_creature = create_creature(&mut game, "Cara Drake", cara);
        let alice_creature = create_creature(&mut game, "Alice Angel", alice);
        let mut dm = NamedObjectDecisionMaker::new(&["bob", "cara", "alice"]);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect =
            DirectionalAdjacentPlayerControlEffect::new(ObjectFilter::creature(), "left", "right");
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.current_controller(bob_creature), Some(alice));
        assert_eq!(game.current_controller(cara_creature), Some(bob));
        assert_eq!(game.current_controller(alice_creature), Some(cara));
    }

    #[test]
    fn directional_control_skips_players_with_no_adjacent_candidate() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.new_object_id();
        game.set_chosen_named_option(source, "right".to_string());
        let bob_creature = create_creature(&mut game, "Bob Bear", bob);
        let alice_creature = create_creature(&mut game, "Alice Angel", alice);
        let mut dm = NamedObjectDecisionMaker::new(&["alice", "bob"]);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        let effect =
            DirectionalAdjacentPlayerControlEffect::new(ObjectFilter::creature(), "left", "right");
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        assert_eq!(game.current_controller(alice_creature), Some(bob));
        assert_eq!(game.current_controller(bob_creature), Some(cara));
    }
}

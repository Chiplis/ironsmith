use crate::decisions::context::{SelectObjectsContext, SelectableObject};
use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::tag::TagKey;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct RearrangeLookedCardsInLibraryEffect {
    pub tag: TagKey,
    pub chooser: PlayerFilter,
    pub count: ChoiceCount,
}

impl RearrangeLookedCardsInLibraryEffect {
    pub fn new(tag: impl Into<TagKey>, chooser: PlayerFilter, count: ChoiceCount) -> Self {
        Self {
            tag: tag.into(),
            chooser,
            count,
        }
    }
}

fn compute_choice_bounds(
    count: ChoiceCount,
    x_value: Option<u32>,
    candidate_count: usize,
) -> (usize, usize) {
    let (min, max) = if count.dynamic_x {
        let x = x_value.unwrap_or(0) as usize;
        let max = x.min(candidate_count);
        let min = if count.up_to_x { 0 } else { max };
        (min, max)
    } else {
        (
            count.min.min(candidate_count),
            count.max.unwrap_or(candidate_count).min(candidate_count),
        )
    };
    (min, max)
}

fn normalize_selected(
    selected: Vec<ObjectId>,
    candidates_top_to_bottom: &[ObjectId],
    min: usize,
    max: usize,
) -> Vec<ObjectId> {
    let mut normalized: Vec<ObjectId> = candidates_top_to_bottom
        .iter()
        .copied()
        .filter(|id| selected.contains(id))
        .take(max)
        .collect();

    if normalized.len() < min {
        for id in candidates_top_to_bottom {
            if normalized.len() >= min {
                break;
            }
            if !normalized.contains(id) {
                normalized.push(*id);
            }
        }
    }

    normalized
}

fn describe_count(min: usize, max: usize) -> String {
    if min == max {
        match max {
            0 => "Choose no cards to leave on top of your library".to_string(),
            1 => "Choose one card to leave on top of your library".to_string(),
            _ => format!("Choose exactly {max} cards to leave on top of your library"),
        }
    } else {
        match max {
            0 => "Choose no cards to leave on top of your library".to_string(),
            1 => "Choose up to one card to leave on top of your library".to_string(),
            _ => format!("Choose up to {max} cards to leave on top of your library"),
        }
    }
}

impl EffectExecutor for RearrangeLookedCardsInLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let Some(snapshots) = ctx.get_tagged_all(self.tag.as_str()) else {
            return Ok(EffectOutcome::resolved());
        };

        let mut cards: Vec<ObjectId> = snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect();
        cards.retain(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.zone == Zone::Library)
        });
        if cards.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let owners: std::collections::HashSet<_> = cards
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.owner))
            .collect();
        if owners.len() != 1 {
            return Ok(EffectOutcome::resolved());
        }
        let library_owner = *owners.iter().next().expect("owner exists");

        let Some(player) = game.player(library_owner) else {
            return Ok(EffectOutcome::resolved());
        };

        let current_top_to_bottom: Vec<ObjectId> = player
            .library
            .iter()
            .rev()
            .copied()
            .filter(|id| cards.contains(id))
            .collect();
        if current_top_to_bottom.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let (min, max) =
            compute_choice_bounds(self.count, ctx.x_value, current_top_to_bottom.len());
        let candidates: Vec<SelectableObject> = current_top_to_bottom
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| SelectableObject::new(id, obj.name.clone()))
            })
            .collect();
        let choice_ctx = SelectObjectsContext::new(
            chooser,
            Some(ctx.source),
            describe_count(min, max),
            candidates,
            min,
            Some(max),
        );
        let selected = ctx.decision_maker.decide_objects(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        let chosen_top_to_bottom = normalize_selected(selected, &current_top_to_bottom, min, max);
        let mut chosen_set = std::collections::HashSet::new();
        chosen_set.extend(chosen_top_to_bottom.iter().copied());

        let mut to_bottom: Vec<ObjectId> = current_top_to_bottom
            .iter()
            .copied()
            .filter(|id| !chosen_set.contains(id))
            .collect();
        game.shuffle_slice(&mut to_bottom);

        let mut rebuilt_library: Vec<ObjectId> = player
            .library
            .iter()
            .copied()
            .filter(|id| !current_top_to_bottom.contains(id))
            .collect();
        rebuilt_library.splice(0..0, to_bottom);
        rebuilt_library.extend(chosen_top_to_bottom.iter().rev().copied());

        if let Some(player) = game.player_mut(library_owner) {
            player.library = rebuilt_library;
        }

        Ok(EffectOutcome::count(current_top_to_bottom.len() as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;

    #[derive(Default)]
    struct ChooseNothingDm;

    impl DecisionMaker for ChooseNothingDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            Vec::new()
        }
    }

    struct ChooseNamedDm {
        name: String,
    }

    impl DecisionMaker for ChooseNamedDm {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter_map(|candidate| {
                    game.object(candidate.id)
                        .and_then(|obj| (obj.name == self.name).then_some(candidate.id))
                })
                .collect()
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_library_card(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), name).build(),
            owner,
            Zone::Library,
        )
    }

    #[test]
    fn choosing_none_moves_all_looked_cards_to_bottom() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let untouched = add_library_card(&mut game, alice, "Untouched");
        let viewed_a = add_library_card(&mut game, alice, "Viewed A");
        let viewed_b = add_library_card(&mut game, alice, "Viewed B");

        let mut dm = ChooseNothingDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(
            "looked",
            vec![
                ObjectSnapshot::from_object(game.object(viewed_a).expect("viewed a"), &game),
                ObjectSnapshot::from_object(game.object(viewed_b).expect("viewed b"), &game),
            ],
        );

        let effect = RearrangeLookedCardsInLibraryEffect::new(
            "looked",
            PlayerFilter::You,
            ChoiceCount::up_to(1),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        let library = game.player(alice).expect("alice").library.clone();
        assert_eq!(library.last().copied(), Some(untouched));
        assert!(library.iter().take(2).any(|id| *id == viewed_a));
        assert!(library.iter().take(2).any(|id| *id == viewed_b));
    }

    #[test]
    fn chosen_card_stays_on_top() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_library_card(&mut game, alice, "Untouched");
        let viewed_a = add_library_card(&mut game, alice, "Viewed A");
        let viewed_b = add_library_card(&mut game, alice, "Viewed B");

        let mut dm = ChooseNamedDm {
            name: "Viewed A".to_string(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(
            "looked",
            vec![
                ObjectSnapshot::from_object(game.object(viewed_a).expect("viewed a"), &game),
                ObjectSnapshot::from_object(game.object(viewed_b).expect("viewed b"), &game),
            ],
        );

        let effect = RearrangeLookedCardsInLibraryEffect::new(
            "looked",
            PlayerFilter::You,
            ChoiceCount::up_to(1),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");

        let library = game.player(alice).expect("alice").library.clone();
        let top = *library.last().expect("top exists");
        let top_name = game.object(top).expect("top object").name.clone();
        assert_eq!(top_name, "Viewed A");
    }
}

//! Look at objects matching a filter.

use crate::decisions::context::ViewCardsContext;
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::helpers::{resolve_player_filter_to_list, view_hidden_candidate_objects};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object_query::candidate_ids_for_filter;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

pub type LookAtObjectsEffect = ironsmith_core::LookAtObjectsEffect;

impl EffectExecutor for LookAtObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let subjects = match resolve_player_filter_to_list(game, &self.subject, &filter_ctx, ctx) {
            Ok(subjects) => subjects,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };
        let viewers = resolve_player_filter_to_list(game, &self.viewer, &filter_ctx, ctx)?;

        if subjects.is_empty() || viewers.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let candidate_ids = candidate_ids_for_filter(game, &self.filter);
        let mut viewed = Vec::new();
        for id in candidate_ids {
            let Some(object) = game.object(id) else {
                continue;
            };
            if self.filter.matches(object, &filter_ctx, game) {
                viewed.push(id);
            }
        }

        if viewed.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let zone = self.filter.zone.unwrap_or(Zone::Battlefield);
        let description = format!("Look at {}", self.filter.description());
        for subject in subjects {
            for viewer in &viewers {
                let view_ctx = ViewCardsContext::new(
                    *viewer,
                    subject,
                    Some(ctx.source),
                    zone,
                    description.clone(),
                );
                ctx.decision_maker
                    .view_cards(game, *viewer, &viewed, &view_ctx);
            }
        }

        remember_hidden_views(game, ctx, &viewed, &viewers);

        let memory = viewed
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|object| ObjectSnapshot::from_object(object, game))
            })
            .map(|snapshot| OutcomeObjectMemory::from_snapshot(&snapshot))
            .collect::<Vec<_>>();
        Ok(EffectOutcome::count(viewed.len() as i32)
            .with_chosen_object_memory(memory.clone())
            .with_affected_object_memory(memory))
    }
}

fn remember_hidden_views(
    game: &GameState,
    ctx: &mut ExecutionContext,
    viewed: &[ObjectId],
    viewers: &[PlayerId],
) {
    for viewer in viewers {
        view_hidden_candidate_objects(game, ctx, *viewer, viewed, "Look at hidden objects", false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::OutcomeValue;
    use crate::effects::ResolvedTarget;
    use crate::filter::ObjectFilter;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::PlayerFilter;
    use crate::types::CardType;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        cards: Vec<ObjectId>,
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<ViewCall>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                cards: cards.to_vec(),
            });
        }
    }

    fn make_creature(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::PowerToughness::fixed(2, 2))
            .build()
    }

    fn add_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature(id.0 as u32, name);
        let object = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(object);
        id
    }

    #[test]
    fn spy_network_looks_at_target_players_face_down_creatures() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let face_down = add_creature(&mut game, "Face-Down Creature", bob);
        let face_up = add_creature(&mut game, "Face-Up Creature", bob);
        let alice_face_down = add_creature(&mut game, "Alice Face-Down Creature", alice);
        game.set_face_down(face_down);
        game.set_face_down(alice_face_down);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let effect = LookAtObjectsEffect::new(
            ObjectFilter::creature()
                .face_down()
                .controlled_by(PlayerFilter::target_player()),
            PlayerFilter::You,
            PlayerFilter::target_player(),
        );

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, OutcomeValue::Count(1));
        assert_eq!(dm.calls.len(), 1);
        assert_eq!(dm.calls[0].viewer, alice);
        assert_eq!(dm.calls[0].subject, bob);
        assert_eq!(dm.calls[0].zone, Zone::Battlefield);
        assert_eq!(dm.calls[0].cards, vec![face_down]);
        assert!(!dm.calls[0].cards.contains(&face_up));
        assert!(!dm.calls[0].cards.contains(&alice_face_down));
    }

    #[test]
    fn spy_network_no_face_down_creatures_branch_views_nothing() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _face_up = add_creature(&mut game, "Face-Up Creature", bob);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let effect = LookAtObjectsEffect::new(
            ObjectFilter::creature()
                .face_down()
                .controlled_by(PlayerFilter::target_player()),
            PlayerFilter::You,
            PlayerFilter::target_player(),
        );

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, OutcomeValue::Count(0));
        assert!(dm.calls.is_empty());
    }
}

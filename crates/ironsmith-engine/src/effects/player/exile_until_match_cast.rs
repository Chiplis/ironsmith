//! Exile cards from the top of a library until one matches a filter, then offer
//! that card to be cast and put the rest on the bottom in random order.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::CompletedEffectOutputs;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{LibraryBottomOrder, LibraryConsultMode};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchCastEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub caster: PlayerFilter,
    pub without_paying_mana_cost: bool,
}

impl ExileUntilMatchCastEffect {
    pub fn new(
        player: PlayerFilter,
        filter: ObjectFilter,
        caster: PlayerFilter,
        without_paying_mana_cost: bool,
    ) -> Self {
        Self {
            player,
            filter,
            caster,
            without_paying_mana_cost,
        }
    }
}

impl EffectExecutor for ExileUntilMatchCastEffect {
    fn canonical_program(&self) -> Option<Effect> {
        use crate::effects::composition::{
            ExactTaggedObjects, OutcomeBranchProgram, OutcomeBranchTest, ScopedProgram,
        };
        use crate::effects::{
            CastTaggedEffect, ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule, MayEffect,
        };
        let owner = TagKey::from("__consult_owner");
        let caster = TagKey::from("__consult_caster");
        let all = TagKey::from("__consult_all");
        let matched = TagKey::from("__consult_match");
        let cast_result = TagKey::from("__consult_cast_result");
        let bound_caster = PlayerFilter::TaggedPlayer(caster.clone());
        let mut cast = CastTaggedEffect::new(matched.clone(), bound_caster.clone());
        cast.without_paying_mana_cost = self.without_paying_mana_cost;
        let offered = Effect::new(OutcomeBranchProgram {
            producer: Effect::new(ExactTaggedObjects {
                tag: matched.clone(),
                zone: crate::zone::Zone::Exile,
            }),
            test: OutcomeBranchTest::HasObjects,
            if_true: vec![Effect::new(MayEffect::new_for_player(
                vec![Effect::new(cast).tag_all(cast_result.clone())],
                bound_caster.clone(),
            ))],
            if_false: vec![],
        });
        Some(Effect::new(ScopedProgram {
            players: vec![
                (owner.clone(), self.player.clone()),
                (caster, self.caster.clone()),
            ],
            object_tags: vec![all.clone(), matched.clone(), cast_result.clone()],
            effects: vec![
                Effect::new(ConsultTopOfLibraryEffect::new(
                    PlayerFilter::TaggedPlayer(owner),
                    LibraryConsultMode::Exile,
                    self.filter.clone(),
                    ConsultTopOfLibraryStopRule::FirstMatch,
                    all.clone(),
                    matched,
                )),
                offered,
                Effect::put_tagged_remainder_on_library_bottom(
                    all,
                    None,
                    LibraryBottomOrder::Random,
                    bound_caster,
                ),
            ],
            result_objects: (cast_result, crate::zone::Zone::Stack),
            retain_result_snapshots: false,
        }))
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.canonical_program()
            .expect("consultation cast program")
            .0
            .execute_child_with_outputs(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn free_consulted_cast_returns_the_stack_arrival_and_cleans_its_bindings() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Consulted spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let original = game.create_object_from_card(&card, player, Zone::Library);
        let mut ctx = ExecutionContext::new_default(source, player);
        let outcome = ExileUntilMatchCastEffect::new(
            PlayerFilter::You,
            ObjectFilter::default().with_type(CardType::Sorcery),
            PlayerFilter::You,
            true,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let copied = outcome.objects().expect("cast result")[0];
        assert_ne!(copied, original);
        assert_eq!(game.object(copied).unwrap().zone, Zone::Stack);
        assert_eq!(game.stack.last().unwrap().object_id, copied);
        assert!(game.player(player).unwrap().library.is_empty());
        assert!(ctx.get_tagged_all("__consult_cast_result").is_none());
    }

    #[derive(Default)]
    struct Decline;
    impl crate::decision::DecisionMaker for Decline {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            false
        }
    }
    #[test]
    fn declining_cast_returns_the_exact_exiled_card_to_its_library() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Declined spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        game.create_object_from_card(&card, player, Zone::Library);
        let mut dm = Decline;
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let outcome = ExileUntilMatchCastEffect::new(
            PlayerFilter::You,
            ObjectFilter::default().with_type(CardType::Sorcery),
            PlayerFilter::You,
            true,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(game.stack.is_empty());
        assert!(game.exile.is_empty());
        assert_eq!(game.player(player).unwrap().library.len(), 1);
    }
}

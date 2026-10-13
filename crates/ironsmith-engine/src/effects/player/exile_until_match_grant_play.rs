//! Exile cards from the top of a library until one matches a filter, then
//! grant temporary play permission for that exiled card until end of turn.

use crate::effect::EffectOutcome;
use crate::effects::CompletedEffectOutputs;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::LibraryConsultMode;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchGrantPlayEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub caster: PlayerFilter,
}

impl ExileUntilMatchGrantPlayEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, caster: PlayerFilter) -> Self {
        Self {
            player,
            filter,
            caster,
        }
    }
}

impl EffectExecutor for ExileUntilMatchGrantPlayEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        use crate::effect::Effect;
        use crate::effects::{
            ConsultTopOfLibraryEffect, ConsultTopOfLibraryStopRule, GrantPlayTaggedDuration,
            GrantPlayTaggedEffect,
        };
        let owner = TagKey::from("__consult_owner");
        let caster = TagKey::from("__consult_caster");
        let all = TagKey::from("__consult_all");
        let matched = TagKey::from("__consult_match");
        let mut grant = GrantPlayTaggedEffect::new(
            matched.clone(),
            PlayerFilter::TaggedPlayer(caster.clone()),
            GrantPlayTaggedDuration::UntilEndOfTurn,
            true,
            false,
        )
        .with_filter(ObjectFilter::default().in_zone(Zone::Exile));
        grant.exact_tagged_objects = true;
        Some(Effect::new(crate::effects::composition::ScopedProgram {
            players: vec![
                (owner.clone(), self.player.clone()),
                (caster, self.caster.clone()),
            ],
            object_tags: vec![all.clone(), matched.clone()],
            effects: vec![
                Effect::new(ConsultTopOfLibraryEffect::new(
                    PlayerFilter::TaggedPlayer(owner),
                    LibraryConsultMode::Exile,
                    self.filter.clone(),
                    ConsultTopOfLibraryStopRule::FirstMatch,
                    all,
                    matched.clone(),
                )),
                Effect::new(grant),
            ],
            result_objects: (matched, Zone::Exile),
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
            .expect("consultation program")
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

    #[test]
    fn consultation_grants_to_the_bound_caster_and_restores_local_tags() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Consulted creature")
            .card_types(vec![CardType::Creature])
            .build();
        let original = game.create_object_from_card(&card, alice, Zone::Library);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_players("__consult_caster", vec![alice]);
        let result = ExileUntilMatchGrantPlayEffect::new(
            PlayerFilter::You,
            ObjectFilter::default().with_type(CardType::Creature),
            PlayerFilter::Specific(bob),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let arrival = result.objects().unwrap()[0];
        assert_ne!(arrival, original);
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);
        assert!(game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            arrival,
            Zone::Exile,
            bob
        ));
        assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            arrival,
            Zone::Exile,
            alice
        ));
        assert_eq!(
            ctx.get_tagged_players("__consult_caster"),
            Some(&vec![alice])
        );
        assert!(ctx.get_tagged_all("__consult_match").is_none());
        assert!(ctx.get_tagged_all("__consult_all").is_none());
    }
}

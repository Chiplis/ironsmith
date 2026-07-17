//! Register a one-shot spell-ability grant for the next matching spell this turn.

use crate::ability::Ability;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type GrantNextSpellAbilityEffect = ironsmith_core::GrantNextSpellAbilityEffect<Ability>;

impl EffectExecutor for GrantNextSpellAbilityEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        game.add_temporary_spell_ability_grant(
            player,
            ctx.source,
            self.filter.clone(),
            self.ability.clone(),
            1,
        );
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn execute_registers_next_spell_ability_grant() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = GrantNextSpellAbilityEffect::new(
            PlayerFilter::You,
            ObjectFilter::noncreature_spell().cast_by(PlayerFilter::You),
            StaticAbility::cascade().into(),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("grant effect should resolve");

        assert_eq!(game.effect_store.temporary_spell_ability_grants.len(), 1);
        let grant = &game.effect_store.temporary_spell_ability_grants[0];
        assert_eq!(grant.player, alice);
        assert!(matches!(
            &grant.ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Cascade
        ));
    }

    #[test]
    fn next_spell_grant_can_match_the_authoritative_cast_origin() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(9010), "Origin-Test Instant")
            .card_types(vec![CardType::Instant])
            .build();
        let hand_id = game.create_object_from_card(&card, alice, Zone::Hand);
        let origin = crate::snapshot::ObjectSnapshot::from_object(
            game.object(hand_id).expect("card in hand"),
            &game,
        );
        let spell_id = game
            .move_object_by_effect(hand_id, Zone::Stack)
            .expect("card should move to the stack");
        game.set_cast_origin_snapshot(spell_id, origin);

        let filter = ObjectFilter::instant_or_sorcery()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You)
            .cast_by(PlayerFilter::You);
        game.add_temporary_spell_ability_grant(
            alice,
            spell_id,
            filter,
            StaticAbility::rebound().into(),
            1,
        );

        assert!(
            game.temporary_granted_spell_abilities(spell_id, alice)
                .iter()
                .any(|ability| matches!(
                    &ability.kind,
                    AbilityKind::Static(static_ability)
                        if static_ability.id() == StaticAbilityId::Rebound
                )),
            "the stack spell should match its hand-origin snapshot"
        );
    }
}

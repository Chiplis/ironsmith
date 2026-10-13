//! Legacy free-cast surface expanded to the shared tagged casting permission.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, GrantPlayTaggedEffect};
use crate::game_state::GameState;
#[cfg(test)]
use crate::grant_registry::GrantSource;

pub type GrantTaggedSpellFreeCastUntilEndOfTurnEffect =
    ironsmith_core::GrantTaggedSpellFreeCastUntilEndOfTurnEffect;

fn program(effect: &GrantTaggedSpellFreeCastUntilEndOfTurnEffect) -> GrantPlayTaggedEffect {
    let mut grant = GrantPlayTaggedEffect::new(
        effect.tag.clone(),
        effect.player.clone(),
        effect.duration,
        false,
        false,
    )
    .with_alternative_cost(crate::cost::TotalCost::from_costs(Vec::new()))
    .while_on_top_of_library_if(effect.while_on_top_of_library);
    if let Some(zone) = effect.zone {
        grant = grant.with_filter(crate::filter::ObjectFilter::default().in_zone(zone));
    }
    grant
}

impl EffectExecutor for GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    fn canonical_program(&self) -> Option<Effect> {
        Some(Effect::new(program(self)))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        program(self).execute_child(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::{GrantPlayTaggedDuration, GrantPlayTaggedEffect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ObjectRef, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn next_turn_free_cast_permission_uses_a_queued_extra_turn() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::from_raw(45), "Extra-turn Exile")
            .card_types(vec![CardType::Sorcery])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(
            game.object(exiled_id).expect("exiled card should exist"),
            &game,
        );
        game.turn_store.extra_turns.push(alice);

        let tag = crate::TagKey::from("next_turn_free_cast");
        let mut tags = std::collections::HashMap::new();
        tags.insert(tag.clone(), vec![snapshot]);
        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(474);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let mut effect = GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(tag, PlayerFilter::You);
        effect.duration = GrantPlayTaggedDuration::UntilYourNextTurnEnd;

        effect
            .execute(&mut game, &mut ctx)
            .expect("next-turn free-cast permission should resolve");

        assert_eq!(
            game.effect_store.grant_registry.grants[0].source,
            GrantSource::until_player_next_turn_end(source, alice, 2)
        );
    }

    #[test]
    fn release_to_the_wind_owner_free_cast_permission_tracks_exiled_card_zone() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let card = CardBuilder::new(CardId::from_raw(46), "Released Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let exiled_id = game.create_object_from_card(&card, bob, Zone::Exile);
        let snapshot = ObjectSnapshot::from_object(
            game.object(exiled_id).expect("exiled card should exist"),
            &game,
        );

        let tag = crate::TagKey::from("release_to_the_wind_exiled");
        let mut tags = std::collections::HashMap::new();
        tags.insert(tag.clone(), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(475);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let owner_filter = PlayerFilter::OwnerOf(ObjectRef::tagged(tag.clone()));

        GrantPlayTaggedEffect::new(
            tag.clone(),
            owner_filter.clone(),
            GrantPlayTaggedDuration::ForAsLongAsExiled,
            false,
            false,
        )
        .execute(&mut game, &mut ctx)
        .expect("Release to the Wind play permission should resolve");
        GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(tag, owner_filter)
            .for_as_long_as_exiled()
            .execute(&mut game, &mut ctx)
            .expect("Release to the Wind free-cast permission should resolve");

        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                bob
            ),
            "the exiled card's owner should be allowed to cast it from exile"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled_id,
                Zone::Exile,
                alice
            ),
            "Release to the Wind grants permission to the card's owner, not the spell controller"
        );

        let alternatives = game
            .effect_store
            .grant_registry
            .granted_alternative_casts_for_card(&game, exiled_id, Zone::Exile, bob);
        assert!(
            alternatives.iter().any(|alternative| matches!(
                &alternative.method,
                AlternativeCastingMethod::FromZone { name, total_cost, zone: Zone::Exile, .. }
                    if name.as_str() == "Without paying its mana cost" && total_cost.costs().is_empty()
            )),
            "the owner should receive a no-mana alternative cast from exile, got {alternatives:?}"
        );

        game.turn.turn_number = game.turn.turn_number.saturating_add(20);
        assert!(
            !game
                .effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, exiled_id, Zone::Exile, bob)
                .is_empty(),
            "the free-cast permission should not expire at end of turn while the card remains exiled"
        );

        let graveyard_id = game
            .move_object_by_effect(exiled_id, Zone::Graveyard)
            .expect("exiled card should move to graveyard");
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                graveyard_id,
                Zone::Graveyard,
                bob
            ),
            "Release to the Wind's play permission should stop outside exile"
        );
        assert!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, graveyard_id, Zone::Graveyard, bob)
                .is_empty(),
            "Release to the Wind's free-cast permission should stop outside exile"
        );

        let reexiled_id = game
            .move_object_by_effect(graveyard_id, Zone::Exile)
            .expect("card should be able to move back to exile");
        let before = game.effect_store.grant_registry.grants.len();
        GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
            "release_to_the_wind_exiled",
            PlayerFilter::You,
        )
        .execute(&mut game, &mut ctx)
        .expect("stale tag is harmless");
        assert_eq!(
            game.effect_store.grant_registry.grants.len(),
            before,
            "a later grant must not follow a stale tag into a new exile incarnation"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                reexiled_id,
                Zone::Exile,
                bob
            ),
            "Release to the Wind's permission should not revive after the card leaves exile"
        );
        assert!(
            game.effect_store
                .grant_registry
                .granted_alternative_casts_for_card(&game, reexiled_id, Zone::Exile, bob)
                .is_empty(),
            "Release to the Wind's free-cast permission should not revive after the card leaves exile"
        );
    }
}

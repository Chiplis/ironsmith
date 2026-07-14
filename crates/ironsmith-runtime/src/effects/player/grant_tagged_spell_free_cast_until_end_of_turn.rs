//! Grant temporary "cast this tagged spell without paying its mana cost"
//! permissions.

use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant_registry::GrantSource;

/// Grants a temporary zero-mana alternative casting method to tagged spells.
pub type GrantTaggedSpellFreeCastUntilEndOfTurnEffect =
    ironsmith_core::GrantTaggedSpellFreeCastUntilEndOfTurnEffect;

impl EffectExecutor for GrantTaggedSpellFreeCastUntilEndOfTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some(snapshots) = ctx.get_tagged_all(self.tag.as_str()).cloned() else {
            return Ok(EffectOutcome::count(0));
        };

        let expires_end_of_turn = match self.duration {
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => game.turn.turn_number,
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                game.turn.turn_number.saturating_add(1)
            }
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
                game.turn.turn_number.saturating_add(1)
            }
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled
            | crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource => u32::MAX,
        };
        let mut granted = 0usize;
        let mut seen = std::collections::HashSet::new();

        for snapshot in snapshots {
            let mut object_id = snapshot.object_id;
            if game.object(object_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                    object_id = found;
                } else {
                    continue;
                }
            }

            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.is_land() || !seen.insert(object_id) {
                continue;
            }
            let zone = object.zone;
            if self.zone.is_some_and(|required_zone| required_zone != zone) {
                continue;
            }
            let source = if self.while_on_top_of_library {
                GrantSource::EffectWhileStableCardOnTopOfLibrary {
                    source_id: ctx.source,
                    expires_end_of_turn,
                    stable_id: object.stable_id,
                    player: object.owner,
                    library_top_revision: game.library_top_revision(object.owner),
                }
            } else {
                GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn,
                }
            };

            let method = AlternativeCastingMethod::alternative_cost(
                "Without paying its mana cost",
                None,
                vec![],
            );
            if self.duration == crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled {
                game.effect_store
                    .grant_registry
                    .grant_alternative_cast_to_stable_card(
                        object_id,
                        object.stable_id,
                        zone,
                        player_id,
                        method,
                        source,
                    );
            } else {
                game.effect_store
                    .grant_registry
                    .grant_alternative_cast_to_card(object_id, zone, player_id, method, source);
            }
            granted += 1;
        }

        Ok(EffectOutcome::count(granted as i32))
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
                alternative.method,
                AlternativeCastingMethod::Composed {
                    name: "Without paying its mana cost",
                    ..
                }
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

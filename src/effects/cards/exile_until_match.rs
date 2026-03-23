//! Exile cards from the top of a library until one matches a filter.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilMatchEffect {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub exiled_tag: Option<TagKey>,
    pub match_tag: Option<TagKey>,
}

impl ExileUntilMatchEffect {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self {
            player,
            filter,
            exiled_tag: None,
            match_tag: None,
        }
    }

    pub fn tag_all_exiled(mut self, tag: impl Into<TagKey>) -> Self {
        self.exiled_tag = Some(tag.into());
        self
    }

    pub fn tag_match(mut self, tag: impl Into<TagKey>) -> Self {
        self.match_tag = Some(tag.into());
        self
    }
}

impl EffectExecutor for ExileUntilMatchEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if let Some(tag) = &self.exiled_tag {
            ctx.clear_object_tag(tag);
        }
        if let Some(tag) = &self.match_tag {
            ctx.clear_object_tag(tag);
        }

        let mut exiled = Vec::new();

        loop {
            let Some(top_card_id) = game
                .player(player_id)
                .and_then(|player| player.library.last().copied())
            else {
                break;
            };

            let Some((exiled_id, final_zone)) = game.move_object_with_commander_options(
                top_card_id,
                Zone::Exile,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                break;
            };
            if final_zone != Zone::Exile {
                continue;
            }

            let Some(snapshot) = game
                .object(exiled_id)
                .map(|object| ObjectSnapshot::from_object(object, game))
            else {
                continue;
            };

            let filter_ctx = ctx.filter_context(game);
            let is_match = game
                .object(exiled_id)
                .is_some_and(|object| self.filter.matches(object, &filter_ctx, game));

            if let Some(tag) = &self.exiled_tag {
                ctx.tag_object(tag.clone(), snapshot.clone());
            }
            exiled.push(exiled_id);

            if !is_match {
                continue;
            }

            if let Some(tag) = &self.match_tag {
                ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
            }
            break;
        }

        if exiled.is_empty() {
            Ok(EffectOutcome::count(0))
        } else {
            Ok(EffectOutcome::with_objects(exiled))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaCost;
    use crate::types::CardType;

    const CHOSEN_NAME_TAG: &str = "chosen_name";
    const EXILED_TAG: &str = "exiled";
    const MATCH_TAG: &str = "match";

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn push_library_card(game: &mut GameState, controller: PlayerId, name: &str) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .build();
        game.create_object_from_card(&card, controller, Zone::Library);
    }

    fn tag_chosen_name(
        game: &GameState,
        ctx: &mut ExecutionContext,
        controller: PlayerId,
        name: &str,
    ) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::new())
            .build();
        let chosen_id = ObjectId::from_raw(7777);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            &crate::object::Object::from_card(chosen_id, &card, controller, Zone::Command),
            game,
        );
        ctx.set_tagged_objects(CHOSEN_NAME_TAG, vec![snapshot]);
    }

    #[test]
    fn exile_until_match_tags_all_exiled_cards_and_the_match() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        push_library_card(&mut game, alice, "Target Card");
        push_library_card(&mut game, alice, "Filler One");
        push_library_card(&mut game, alice, "Filler Two");

        let source = ObjectId::from_raw(999);
        let mut ctx = ExecutionContext::new_default(source, alice);
        tag_chosen_name(&game, &mut ctx, alice, "Target Card");

        let effect = ExileUntilMatchEffect::new(
            PlayerFilter::You,
            ObjectFilter::default().match_tagged(
                CHOSEN_NAME_TAG,
                crate::target::TaggedOpbjectRelation::SameNameAsTagged,
            ),
        )
        .tag_all_exiled(EXILED_TAG)
        .tag_match(MATCH_TAG);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(outcome.output_objects().len(), 3);
        assert_eq!(
            ctx.get_tagged_all(EXILED_TAG)
                .expect("exiled cards should be tagged")
                .len(),
            3
        );
        assert_eq!(
            ctx.get_tagged(MATCH_TAG)
                .expect("matching card should be tagged")
                .name,
            "Target Card"
        );
        assert_eq!(game.player(alice).expect("alice exists").library.len(), 0);
    }

    #[test]
    fn exile_until_match_exhausts_the_library_when_no_card_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        push_library_card(&mut game, alice, "Filler One");
        push_library_card(&mut game, alice, "Filler Two");

        let source = ObjectId::from_raw(1000);
        let mut ctx = ExecutionContext::new_default(source, alice);
        tag_chosen_name(&game, &mut ctx, alice, "Target Card");

        let effect = ExileUntilMatchEffect::new(
            PlayerFilter::You,
            ObjectFilter::default().match_tagged(
                CHOSEN_NAME_TAG,
                crate::target::TaggedOpbjectRelation::SameNameAsTagged,
            ),
        )
        .tag_all_exiled(EXILED_TAG)
        .tag_match(MATCH_TAG);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(outcome.output_objects().len(), 2);
        assert!(ctx.get_tagged(MATCH_TAG).is_none());
        assert_eq!(
            ctx.get_tagged_all(EXILED_TAG)
                .expect("all exiled cards should be tagged")
                .len(),
            2
        );
        assert_eq!(game.player(alice).expect("alice exists").library.len(), 0);
    }
}

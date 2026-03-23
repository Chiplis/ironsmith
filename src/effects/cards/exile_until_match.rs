//! Exile cards from the top of a library until one matches a filter.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{LibraryConsultMode, LibraryConsultStopRule, execute_library_consult};
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

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
        let filter_ctx = ctx.filter_context(game);
        let result = execute_library_consult(
            game,
            ctx,
            player_id,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::FirstMatch,
            self.exiled_tag.as_ref(),
            self.match_tag.as_ref(),
            |object, game| self.filter.matches(object, &filter_ctx, game),
        )?;

        if result.exposed_object_ids.is_empty() {
            Ok(EffectOutcome::count(0))
        } else {
            Ok(EffectOutcome::with_objects(result.exposed_object_ids))
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
    use crate::zone::Zone;

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

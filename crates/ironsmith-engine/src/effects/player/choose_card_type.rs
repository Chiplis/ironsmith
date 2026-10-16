//! Choose a card type and store it on the source object.

use crate::effects::composition::scoped_choice::{select_typed_option, SingleChoicePolicy};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::types::CardType;
pub use ironsmith_core::ChooseCardTypeEffect;

impl EffectExecutor for ChooseCardTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let mut options = self.card_type_options();
        if let Some((filter, count)) = &self.shared_among {
            let objects = crate::effects::helpers::resolve_objects_from_spec(
                game, &crate::target::ChooseSpec::All(filter.clone()), ctx,
            )?.into_iter().collect::<std::collections::HashSet<_>>();
            options.retain(|card_type| {
                objects.iter().filter(|id| game.current_card_types(**id).is_some_and(|types| types.contains(card_type))).count()
                    >= (*count).max(1) as usize
            });
        }
        if options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let permanent_type_options = [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ];
        let prompt = if options == permanent_type_options {
            "Choose a permanent type"
        } else {
            "Choose a card type"
        };
        let menu = options.iter().map(|value| (*value, value.to_string())).collect::<Vec<_>>();
        let Some(chosen) = select_typed_option(game, ctx, chooser, prompt, &menu, SingleChoicePolicy::First)? else {
            return Ok(EffectOutcome::count(0));
        };
        game.set_chosen_card_type(ctx.source, chosen);
        Ok(EffectOutcome::count(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::ids::{ObjectId, PlayerId};
    use crate::test_prelude::*;

    struct ChooseLandDm;
    impl DecisionMaker for ChooseLandDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("land"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    #[test]
    fn choose_card_type_effect_stores_selected_type_on_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::new();
        let mut dm = ChooseLandDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ChooseCardTypeEffect::new(PlayerFilter::You, vec![CardType::Creature, CardType::Land])
            .execute(&mut game, &mut ctx)
            .expect("choose-card-type should execute");

        assert_eq!(game.chosen_card_type(source), Some(CardType::Land));
    }

    #[test]
    fn shared_card_type_choice_counts_distinct_matching_objects_and_keeps_empty_domains_empty() {
        #[derive(Default)]
        struct Menu { offered: Vec<String> }
        impl DecisionMaker for Menu {
            fn decide_options(&mut self, _: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                self.offered = ctx.options.iter().map(|option| option.description.to_ascii_lowercase()).collect();
                vec![ctx.options.last().unwrap().index]
            }
        }
        for (first, second, expected) in [
            (vec![CardType::Artifact, CardType::Creature], vec![CardType::Artifact, CardType::Creature], vec!["artifact", "creature"]),
            (vec![CardType::Artifact, CardType::Creature], vec![CardType::Creature], vec!["creature"]),
            (vec![CardType::Kindred, CardType::Instant], vec![CardType::Kindred, CardType::Sorcery], vec!["kindred"]),
            (vec![CardType::Artifact], vec![CardType::Creature], vec![]),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId::from_index(0);
            let source = ObjectId::new();
            for types in [first, second] {
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Type witness")
                    .card_types(types).build();
                let id = game.create_object_from_card(&card, alice, crate::zone::Zone::Exile);
                game.add_exiled_with_source_link(source, id);
                game.add_exiled_with_source_link(source, id); // Repeated links never count twice.
            }
            let unrelated = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Unrelated")
                .card_types(vec![CardType::Land, CardType::Artifact]).build();
            game.create_object_from_card(&unrelated, alice, crate::zone::Zone::Exile);
            let filter = crate::target::ObjectFilter::default().in_zone(crate::zone::Zone::Exile)
                .match_tagged(crate::tag::SOURCE_EXILED_TAG, crate::filter::TaggedOpbjectRelation::IsTaggedObject);
            let mut dm = Menu::default();
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ChooseCardTypeEffect::new(PlayerFilter::You, vec![]).shared_among(filter, 2)
                .execute(&mut game, &mut ctx).unwrap();
            drop(ctx);
            assert_eq!(dm.offered, expected);
            assert_eq!(game.chosen_card_type(source).is_some(), !expected.is_empty());
        }
    }
}

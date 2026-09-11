use super::*;

struct VoteSplit {
    strength_mask: usize,
    voters: Vec<crate::ids::PlayerId>,
}
impl crate::decision::DecisionMaker for VoteSplit {
    fn decide_options(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        self.voters.push(ctx.player);
        let index = game
            .players
            .iter()
            .position(|player| player.id == ctx.player)
            .unwrap();
        let label = if self.strength_mask & (1 << index) != 0 {
            "strength"
        } else {
            "numbers"
        };
        vec![
            ctx.options
                .iter()
                .find(|option| option.legal && option.description.eq_ignore_ascii_case(label))
                .expect("named vote option")
                .index,
        ]
    }
}

#[test]
fn paired_vote_outcomes_count_their_own_options_in_player_order() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Vote outcome fixture")
        .card_types(vec![CardType::Creature])
        .parse_text("Council's dilemma — When this creature enters, starting with you, each player votes for strength or numbers. Put a +1/+1 counter on this creature for each strength vote and create a 1/1 white Soldier creature token for each numbers vote.").unwrap();
    let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("entry trigger");
    };
    for mask in 0usize..8 {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let unrelated =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Existing Soldier")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Soldier])
                .build();
        let other = game.create_object_from_definition(&unrelated, alice, Zone::Battlefield);
        let mut decisions = VoteSplit {
            strength_mask: mask,
            voters: vec![],
        };
        let mut ctx = crate::effects::EffectContext::new_default(source, bob)
            .with_decision_maker(&mut decisions);
        for segment in &triggered.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        drop(ctx);
        assert_eq!(decisions.voters, vec![bob, carol, alice]);
        let strength = mask.count_ones();
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&crate::object::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            strength,
            "mask={mask}"
        );
        let soldiers: Vec<_> = game
            .battlefield
            .iter()
            .copied()
            .filter(|id| *id != other && game.current_has_subtype(*id, Subtype::Soldier))
            .collect();
        assert_eq!(soldiers.len(), 3 - strength as usize, "mask={mask}");
        assert!(
            soldiers
                .iter()
                .all(|id| game.controller_of_id(*id) == Some(bob))
        );
        for soldier in soldiers {
            assert_eq!(game.current_power(soldier), Some(1));
            assert_eq!(game.current_toughness(soldier), Some(1));
            assert_eq!(
                game.current_colors(soldier),
                Some(crate::color::ColorSet::WHITE)
            );
        }
        assert!(game.object(other).unwrap().counters.is_empty());
    }
}

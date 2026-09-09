use super::*;
const TEXT: &str = "Roll two d10 and choose one result. Return a creature card from your graveyard to the battlefield with a number of +1/+1 counters on it equal to that result. Then each opponent loses X life and you gain X life, where X is the other result.";
struct Decisions {
    result: usize,
}
impl crate::decision::DecisionMaker for Decisions {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(ctx.options.len(), 2);
        vec![self.result]
    }
    fn decide_objects(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .take(1)
            .map(|c| c.id)
            .collect()
    }
}
#[test]
fn chosen_die_entry_counters_and_other_result_life() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Grave Endeavor")
        .card_types(vec![CardType::Instant])
        .parse_text(TEXT)
        .unwrap();
    for result in 0..2 {
        for available in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            if available {
                let card =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Returned Creature")
                        .card_types(vec![CardType::Creature])
                        .build();
                game.create_object_from_card(&card, alice, Zone::Graveyard);
            }
            game.force_next_die_roll(3);
            game.force_next_die_roll(8);
            let mut decisions = Decisions { result };
            let mut ctx = crate::effects::EffectContext::new(source, alice, &mut decisions);
            for segment in &definition.spell_effect.as_ref().unwrap().segments {
                for effect in &segment.default_effects {
                    crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                }
            }
            let chosen = [3, 8][result];
            let other = [8, 3][result];
            assert_eq!(game.battlefield.len(), usize::from(available));
            if available {
                assert_eq!(
                    game.counter_count(game.battlefield[0], CounterType::PlusOnePlusOne),
                    chosen
                );
            }
            assert_eq!(game.players[0].life, 20 + other);
            assert_eq!(game.players[1].life, 20 - other);
            assert_eq!(game.players[2].life, 20 - other);
        }
    }
    let program = format!("{:?}", definition.spell_effect.as_ref().unwrap());
    assert!(
        !program.contains("PutCountersEffect"),
        "entry counters must belong to the return, not a later action: {program}"
    );
    assert!(program.contains("BattlefieldEntryCounterSpec"), "{program}");
}
#[test]
fn chosen_die_entry_counters_preserve_both_result_references() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Grave Endeavor")
        .card_types(vec![CardType::Instant])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.replace("Return a creature", "You return a creature")
    );
}

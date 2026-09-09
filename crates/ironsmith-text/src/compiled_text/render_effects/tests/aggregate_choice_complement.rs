use super::*;

struct KeepPowerGroup {
    initial: Vec<crate::ids::ObjectId>,
    chosen: Vec<crate::ids::ObjectId>,
    players: Vec<crate::ids::PlayerId>,
    limit: i32,
}
impl crate::decision::DecisionMaker for KeepPowerGroup {
    fn decide_objects(&mut self, game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
        assert!(self.initial.iter().all(|id| game.object(*id).is_some()),
            "every player's choices must precede the first sacrifice");
        assert_eq!(ctx.aggregate_constraint.as_ref(), Some(&crate::effect::ChoiceAggregateConstraint::total_power_at_most(self.limit)));
        self.players.push(ctx.player);
        ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| {
            assert_eq!(game.controller_of(game.object(candidate.id).unwrap()), ctx.player);
            assert!(game.current_is_creature(candidate.id));
            candidate.id
        }).filter(|id| self.chosen.contains(id)).collect()
    }
}

#[test]
fn aggregate_choice_complement_keeps_the_chosen_sets_and_finishes_all_choices_first() {
    for limit in [2, 4] {
        let oracle = format!("Each player chooses any number of creatures they control with total power {limit} or less, then sacrifices all other creatures they control.");
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Power Choice Probe")
            .card_types(vec![CardType::Sorcery]).parse_text(&oracle).unwrap();
        for empty in [false, true] {
            let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let players = game.players.iter().map(|player| player.id).collect::<Vec<_>>();
            let mut initial = Vec::new();
            let mut chosen = Vec::new();
            for (index, player) in players.iter().enumerate() {
                for power in [1, 1, 3, 5] {
                    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Group Creature")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(power, 2)).build();
                    let id = game.create_object_from_card(&card, *player, Zone::Battlefield);
                    initial.push(id);
                    if !empty && ((index == 0 && power == 1) || (index == 1 && limit == 4 && power == 3)) { chosen.push(id); }
                }
            }
            let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Untouched Land")
                .card_types(vec![CardType::Land]).build();
            let land_id = game.create_object_from_card(&land, players[0], Zone::Battlefield);
            let source = game.create_object_from_definition(&definition, players[1], Zone::Stack);
            let mut decision = KeepPowerGroup { initial: initial.clone(), chosen: chosen.clone(), players: Vec::new(), limit };
            let mut ctx = crate::effects::EffectContext::new_default(source, players[1]).with_decision_maker(&mut decision);
            for segment in &definition.spell_effect.as_ref().unwrap().segments {
                for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            }
            drop(ctx);
            assert_eq!(decision.players, players, "choices follow active-player turn order, not source controller order");
            for id in initial { assert_eq!(game.object(id).is_some(), chosen.contains(&id)); }
            assert!(game.object(land_id).is_some());
        }
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [oracle]);
    }
}

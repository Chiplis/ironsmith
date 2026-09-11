use super::*;
const FIRST: &str = "Choose an opponent. You and that player each sacrifice a creature. Each player who sacrificed a creature this way draws two cards.";
const SECOND: &str = "Choose an opponent. Return a creature card from your graveyard to the battlefield, then that player returns a creature card from their graveyard to the battlefield.";
struct JointChoiceTiming(Vec<crate::ids::ObjectId>, Vec<crate::ids::PlayerId>);
impl crate::decision::DecisionMaker for JointChoiceTiming {
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        choice: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        assert!(
            self.0.iter().all(|id| game.battlefield.contains(id)),
            "all joint sacrifice choices must precede the first zone change"
        );
        self.1.push(choice.player);
        choice
            .candidates
            .iter()
            .filter(|c| c.legal)
            .take(choice.min)
            .map(|c| c.id)
            .collect()
    }
}
#[test]
fn chosen_pair_each_sacrifices_and_only_successful_players_draw() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Offering")
        .card_types(vec![CardType::Sorcery])
        .parse_text(FIRST)
        .unwrap();
    for own_creature in [false, true] {
        for opponent_creature in [false, true] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Charlie".into()],
                20,
            );
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
            let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library card")
                .card_types(vec![CardType::Land])
                .build();
            let mut originals = Vec::new();
            for (index, present) in [own_creature, opponent_creature, true]
                .into_iter()
                .enumerate()
            {
                let owner = game.players[index].id;
                if present {
                    originals.push((
                        index,
                        game.create_object_from_card(&creature, owner, Zone::Battlefield),
                    ));
                }
                for _ in 0..3 {
                    game.create_object_from_card(&land, owner, Zone::Library);
                }
            }
            let mut decision =
                JointChoiceTiming(originals.iter().map(|(_, id)| *id).collect(), vec![]);
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_decision_maker(&mut decision);
            for effect in definition.spell_effect.as_ref().unwrap() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            for (index, id) in originals {
                let zone = game.object(id).map(|o| o.zone);
                assert_eq!(
                    zone == Some(Zone::Battlefield),
                    index == 2,
                    "player {index}, own={own_creature}, opponent={opponent_creature}"
                );
            }
            for (index, present) in [own_creature, opponent_creature, false]
                .into_iter()
                .enumerate()
            {
                assert_eq!(
                    game.players[index].hand.len(),
                    if present { 2 } else { 0 },
                    "draw for player {index}"
                );
            }
        }
    }
}
#[test]
fn chosen_pair_sacrifice_and_return_preserve_their_players() {
    let text = format!("{FIRST}\n{SECOND}");
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Infernal Offering")
            .card_types(vec![CardType::Sorcery])
            .parse_text(&text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn chosen_pair_locks_sacrifice_choices_before_either_player_sacrifices() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Offering")
        .card_types(vec![CardType::Sorcery])
        .parse_text(FIRST)
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let lock = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Sacrifice Lock")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::restriction(
                crate::effect::Restriction::be_sacrificed(
                    ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
                ),
                "Creatures your opponents control can't be sacrificed".into(),
            ),
        ))
        .build();
    let lock = game.create_object_from_definition(&lock, alice, Zone::Battlefield);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Bob creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let other = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library card")
        .card_types(vec![CardType::Land])
        .build();
    for owner in [alice, bob] {
        for _ in 0..3 {
            game.create_object_from_card(&land, owner, Zone::Library);
        }
    }
    game.update_cant_effects();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice);
    for e in definition.spell_effect.as_ref().unwrap() {
        crate::effects::execute_effect(&mut game, e, &mut ctx).unwrap();
    }
    assert!(!game.battlefield.contains(&lock));
    assert!(
        game.battlefield.contains(&other),
        "removing the lock cannot create a new sacrifice choice for Bob"
    );
    assert_eq!(game.players[0].hand.len(), 2);
    assert_eq!(game.players[1].hand.len(), 0);
}

#[test]
fn chosen_pair_makes_both_nontrivial_choices_before_sacrificing() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Offering")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose an opponent. You and that player each sacrifice a creature.")
        .unwrap();
    for active in [0, 1] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.turn.active_player = game.players[active].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Choice")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let mut originals = Vec::new();
        for owner in [alice, bob] {
            for _ in 0..2 {
                originals.push(game.create_object_from_card(&card, owner, Zone::Battlefield));
            }
        }
        let mut dm = JointChoiceTiming(originals, vec![]);
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        for e in definition.spell_effect.as_ref().unwrap() {
            crate::effects::execute_effect(&mut game, e, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(
            dm.1,
            if active == 0 {
                vec![alice, bob]
            } else {
                vec![bob, alice]
            }
        );
        assert_eq!(game.players[0].graveyard.len(), 1);
        assert_eq!(game.players[1].graveyard.len(), 1);
    }
}

struct OfferingOpponents {
    first: usize,
    calls: usize,
}
impl crate::decision::DecisionMaker for OfferingOpponents {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        choice: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(choice.options.len(), 2);
        let selected = if self.calls == 0 {
            self.first
        } else {
            1 - self.first
        };
        self.calls += 1;
        vec![selected]
    }
}
#[test]
fn offering_uses_independent_opponents_for_sacrifices_and_returns() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Infernal Offering")
            .card_types(vec![CardType::Sorcery])
            .parse_text(&format!("{FIRST}\n{SECOND}"))
            .unwrap();
    for first in [0, 1] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Charlie".into()],
            20,
        );
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Library")
            .card_types(vec![CardType::Land])
            .build();
        for index in 0..3 {
            let player = game.players[index].id;
            game.create_object_from_card(&card, player, Zone::Battlefield);
            if index > 0 {
                game.create_object_from_card(&card, player, Zone::Graveyard);
            }
            for _ in 0..3 {
                game.create_object_from_card(&land, player, Zone::Library);
            }
        }
        let mut dm = OfferingOpponents { first, calls: 0 };
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
        for e in definition.spell_effect.as_ref().unwrap() {
            crate::effects::execute_effect(&mut game, e, &mut ctx).unwrap();
        }
        drop(ctx);
        assert_eq!(dm.calls, 2);
        for index in 0..3 {
            let sacrifices = index == 0 || index == first + 1;
            assert_eq!(
                game.players[index].hand.len(),
                if sacrifices { 2 } else { 0 },
                "draws for player {index}, first={first}"
            );
            assert_eq!(
                game.players[index].graveyard.len(),
                if index == first + 1 { 2 } else { 0 },
                "graveyard for player {index}, first={first}"
            );
        }
    }
}

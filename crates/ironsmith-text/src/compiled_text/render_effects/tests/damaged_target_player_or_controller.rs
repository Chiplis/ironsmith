use super::*;

const TEXT: &str = "+3: Destroy target noncreature permanent.\n−2: Gain control of target creature.\n−9: Nicol Bolas deals 7 damage to target player or planeswalker. That player or that planeswalker's controller discards seven cards, then sacrifices seven permanents of their choice.";

struct VictimChoices {
    victim: crate::ids::PlayerId,
    chosen_permanents: Vec<crate::ids::ObjectId>,
}
impl crate::decision::DecisionMaker for VictimChoices {
    fn decide_objects(
        &mut self,
        game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<crate::ids::ObjectId> {
        assert_eq!(
            ctx.player, self.victim,
            "the damaged player or planeswalker's controller chooses"
        );
        let candidates = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        for &id in &candidates {
            assert_eq!(
                game.controller_of_id(id),
                Some(self.victim),
                "choice {} offers {:?} {:?} ({})",
                ctx.description,
                id,
                game.object(id).unwrap().zone,
                game.object(id).unwrap().name
            );
        }
        let chosen = candidates
            .into_iter()
            .filter(|id| !game.current_has_card_type(*id, CardType::Planeswalker))
            .take(ctx.max.unwrap_or(ctx.min))
            .collect::<Vec<_>>();
        if chosen
            .first()
            .is_some_and(|id| game.battlefield.contains(id))
        {
            self.chosen_permanents.extend_from_slice(&chosen);
        }
        chosen
    }
}

#[test]
fn damaged_target_player_or_controller_discards_and_sacrifices_only_seven() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nicol Bolas, Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    let crate::ability::AbilityKind::Activated(ultimate) =
        &definition.abilities.last().unwrap().kind
    else {
        panic!("loyalty ultimate");
    };
    for (target_is_planeswalker, hand_size, permanent_count) in [
        (false, 8usize, 9usize),
        (true, 8, 9),
        (false, 0, 9),
        (true, 0, 9),
        (false, 8, 3),
        (false, 0, 0),
    ] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let carol = game.players[2].id;
        let victim = if target_is_planeswalker { carol } else { bob };
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.add_counters(source, CounterType::Loyalty, 20);
        let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let mut permanents = Vec::new();
        for player in [bob, carol] {
            for _ in 0..hand_size {
                game.create_object_from_card(&land, player, Zone::Hand);
            }
            for _ in 0..permanent_count {
                let id = game.create_object_from_card(&land, player, Zone::Battlefield);
                permanents.push((player, id));
            }
        }
        let walker = if target_is_planeswalker {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Target Walker")
                .card_types(vec![CardType::Planeswalker])
                .build();
            let id = game.create_object_from_card(&card, bob, Zone::Battlefield);
            game.set_current_controller(id, carol);
            game.add_counters(id, CounterType::Loyalty, 10);
            Some(id)
        } else {
            None
        };
        game.refresh_continuous_state();
        let target = walker
            .map(crate::effects::ResolvedTarget::Object)
            .unwrap_or(crate::effects::ResolvedTarget::Player(bob));
        let mut choices = VictimChoices {
            victim,
            chosen_permanents: vec![],
        };
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![target])
            .with_decision_maker(&mut choices);
        ctx.snapshot_targets(&game);
        for segment in &ultimate.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        drop(ctx);
        assert_eq!(choices.chosen_permanents.len(), permanent_count.min(7));
        for (player, id) in permanents {
            assert_eq!(
                game.battlefield.contains(&id),
                !choices.chosen_permanents.contains(&id),
                "only selected permanents are sacrificed ({player:?})"
            );
        }
        for player in [bob, carol] {
            assert_eq!(
                game.player(player).unwrap().hand.len(),
                if player == victim {
                    hand_size.saturating_sub(7)
                } else {
                    hand_size
                }
            );
        }
        if let Some(walker) = walker {
            assert_eq!(game.counter_count(walker, CounterType::Loyalty), 3);
            assert_eq!(game.player(carol).unwrap().life, 20);
        } else {
            assert_eq!(game.player(bob).unwrap().life, 13);
        }
    }
}

#[test]
fn damaged_target_player_or_controller_renders_the_shared_actor() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Nicol Bolas, Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT.replace("Nicol Bolas deals", "This planeswalker deals")
    );
}

#[test]
fn damaged_target_player_or_controller_renderer_rejects_a_different_sacrificing_player() {
    let damage = Effect::new(crate::effects::DealDamageEffect::new(
        3,
        ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any),
    ));
    let discard = Effect::new(crate::effects::DiscardEffect::new(
        2,
        PlayerFilter::TargetPlayerOrControllerOfTarget,
        false,
    ));
    for player in [
        PlayerFilter::TargetPlayerOrControllerOfTarget,
        PlayerFilter::You,
    ] {
        let sacrifice = Effect::new(crate::effects::zones::SacrificePlayerEffect::new(
            ObjectFilter::permanent(),
            1,
            player.clone(),
        ));
        let rendered =
            crate::compiled_text::render_effects::describe_player_damage_then_same_player_discards(
                &[&damage, &discard, &sacrifice],
            );
        assert_eq!(
            rendered.is_some(),
            player == PlayerFilter::TargetPlayerOrControllerOfTarget
        );
    }
}

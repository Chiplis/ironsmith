use super::*;
const TEXT: &str = "When this enchantment enters, you take the initiative and create a Treasure token.\nWhenever you attack the player who has the initiative, create a Treasure token.\nLoud Ruckus — Whenever you complete a dungeon, create a 5/5 red Dragon creature token with flying.";

#[test]
fn initiative_and_token_coordination_executes_both_entry_actions_once() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Loot Dispute")
        .card_types(vec![CardType::Enchantment])
        .parse_text(TEXT)
        .unwrap();
    for previous_holder in [None, Some(0), Some(1)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        game.set_initiative(previous_holder.map(|index| game.players[index].id));
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::ZoneChangeEvent::with_cause(
                source,
                Zone::Stack,
                Zone::Battlefield,
                crate::events::cause::EventCause::effect(),
                Some(crate::snapshot::ObjectSnapshot::from_object(
                    game.object(source).unwrap(),
                    &game,
                )),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let triggers = crate::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), 1);
        let mut ctx =
            crate::effects::EffectContext::new_default(source, alice).with_triggering_event(event);
        for segment in &triggers[0].ability.effects.segments {
            for effect in &segment.default_effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        assert_eq!(game.initiative, Some(alice));
        let tokens = game
            .battlefield
            .iter()
            .filter(|id| game.object(**id).unwrap().kind == crate::object::ObjectKind::Token)
            .collect::<Vec<_>>();
        assert_eq!(tokens.len(), 1);
        assert_eq!(game.controller_of_id(*tokens[0]), Some(alice));
        assert!(game.current_has_subtype(*tokens[0], Subtype::Treasure));
    }
}

#[test]
fn initiative_and_token_coordination_renders_one_shared_subject() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Loot Dispute")
        .card_types(vec![CardType::Enchantment])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn initiative_and_token_coordination_does_not_merge_different_players() {
    for initiative_player in [PlayerFilter::You, PlayerFilter::Opponent] {
        for token_player in [PlayerFilter::You, PlayerFilter::Opponent] {
            let mut create = crate::effects::CreateTokenEffect::new(
                crate::cards::tokens::treasure_token_definition(),
                1,
                token_player.clone(),
            );
            create.actor_surface_explicit = true;
            let effects = [
                Effect::new(crate::effects::TakeInitiativeEffect::new(
                    initiative_player.clone(),
                )),
                Effect::new(create),
            ];
            let rendered = crate::compiled_text::render_effects::effect_lists::describe_you_action_and_create_token(&effects);
            assert_eq!(
                rendered.is_some(),
                initiative_player == PlayerFilter::You && token_player == PlayerFilter::You
            );
        }
    }
}

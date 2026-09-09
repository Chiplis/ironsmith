use super::*;
const TEXT: &str = "Choose an opponent. You and that player each create an X/X green Treefolk creature token.\nChoose an opponent. You and that player each create X 1/1 green Elf Warrior creature tokens.";
fn definition(text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Shared tokens")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .unwrap()
}
struct Choices {
    names: [&'static str; 2],
    calls: usize,
}
impl crate::decision::DecisionMaker for Choices {
    fn decide_options(
        &mut self,
        _: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(ctx.options.iter().filter(|o| o.legal).count(), 2);
        let name = self.names[self.calls];
        self.calls += 1;
        vec![
            ctx.options
                .iter()
                .find(|o| o.legal && o.description == name)
                .unwrap()
                .index,
        ]
    }
}
#[test]
fn shared_dynamic_tokens_preserve_independent_opponents_and_counts() {
    let definition = definition(TEXT);
    for x in [0, 1, 3] {
        for names in [["Bob", "Carol"], ["Carol", "Bob"], ["Bob", "Bob"]] {
            let mut game = crate::game_state::GameState::new(
                vec!["Alice".into(), "Bob".into(), "Carol".into()],
                20,
            );
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let mut choices = Choices { names, calls: 0 };
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_decision_maker(&mut choices);
            ctx.x_value = Some(x);
            for effect in definition.spell_effect.as_ref().unwrap() {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            drop(ctx);
            assert_eq!(choices.calls, 2);
            for player in &game.players {
                let treefolk: Vec<_> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.controller_of_id(*id) == Some(player.id)
                            && game.current_has_subtype(*id, Subtype::Treefolk)
                    })
                    .collect();
                let elves: Vec<_> = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.controller_of_id(*id) == Some(player.id)
                            && game.current_has_subtype(*id, Subtype::Elf)
                    })
                    .collect();
                assert_eq!(
                    treefolk.len(),
                    usize::from(player.id == alice || player.name == names[0])
                );
                assert_eq!(
                    elves.len(),
                    if player.id == alice || player.name == names[1] {
                        x as usize
                    } else {
                        0
                    }
                );
                for id in treefolk {
                    assert_eq!(game.calculated_power(id), Some(x as i32));
                    assert_eq!(game.calculated_toughness(id), Some(x as i32));
                }
                for id in elves {
                    assert_eq!(game.calculated_power(id), Some(1));
                    assert_eq!(game.calculated_toughness(id), Some(1));
                    assert!(game.current_has_subtype(id, Subtype::Warrior));
                }
            }
        }
    }
}
#[test]
fn shared_dynamic_tokens_render_shared_creation_for_generic_blueprints() {
    for (original, replacement) in [
        ("Treefolk", "Treefolk"),
        ("Treefolk", "Beast"),
        ("green", "blue"),
    ] {
        let text = TEXT.replace(original, replacement);
        let rendered = crate::compiled_text::compiled_text_lines(&definition(&text)).join("\n");
        assert!(
            rendered.contains(&format!(
                "You and that player each create an X/X {} creature token",
                if original == "green" {
                    "blue Treefolk".to_string()
                } else {
                    format!("green {replacement}")
                }
            )),
            "{rendered}"
        );
    }
}

#[test]
fn shared_dynamic_tokens_reject_different_sizes_targets_and_blueprints() {
    let definition = definition(TEXT);
    let sequence = definition
        .spell_effect
        .as_ref()
        .unwrap()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::SequenceEffect>())
        .unwrap();
    assert!(describe_shared_dynamic_token_pair(&sequence.effects).is_some());
    for variant in 0..5 {
        let mut effects = sequence.effects.clone();
        if variant < 3 {
            let mut tagged = effects[3]
                .downcast_ref::<crate::effects::TaggedEffect>()
                .unwrap()
                .clone();
            let mut set = tagged
                .effect
                .downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()
                .unwrap()
                .clone();
            match variant {
                0 => set.power = Value::Fixed(5),
                1 => {
                    set.target = ChooseSpec::Tagged(
                        effects[0]
                            .downcast_ref::<crate::effects::TaggedEffect>()
                            .unwrap()
                            .tag
                            .clone(),
                    )
                }
                _ => set.duration = Until::EndOfTurn,
            }
            *tagged.effect = Effect::new(set);
            effects[3] = Effect::new(tagged);
        } else {
            let mut tagged = effects[2]
                .downcast_ref::<crate::effects::TaggedEffect>()
                .unwrap()
                .clone();
            let mut create = tagged
                .effect
                .downcast_ref::<crate::effects::CreateTokenEffect>()
                .unwrap()
                .clone();
            if variant == 3 {
                create.token.card.subtypes = vec![Subtype::Beast];
            } else {
                create.enters_tapped = true;
            }
            *tagged.effect = Effect::new(create);
            effects[2] = Effect::new(tagged);
        }
        assert!(
            describe_shared_dynamic_token_pair(&effects).is_none(),
            "variant {variant}"
        );
    }
}

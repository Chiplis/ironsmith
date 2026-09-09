use super::*;
const TEXT: &str = "At the beginning of combat on your turn, you may cast an instant or sorcery spell with mana value X or less from your hand without paying its mana cost, where X is twice the number of legendary Wizards you control.";
struct AcceptCast;
impl crate::decision::DecisionMaker for AcceptCast {
    fn decide_boolean(
        &mut self,
        _: &crate::game_state::GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}
#[test]
fn scaled_free_cast_uses_twice_only_your_legendary_wizards() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scaling Cast Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    for count in [0u32, 1, 2] {
        for over_limit in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            for (owner, legendary, subtype, copies) in [
                (alice, true, Subtype::Wizard, count),
                (bob, true, Subtype::Wizard, 3),
                (alice, false, Subtype::Wizard, 3),
                (alice, true, Subtype::Warrior, 3),
            ] {
                for index in 0..copies {
                    let card = crate::card::CardBuilder::new(
                        crate::ids::CardId::new(),
                        format!("Count Probe {owner:?} {legendary} {subtype:?} {index}"),
                    )
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![subtype])
                    .supertypes(if legendary {
                        vec![crate::types::Supertype::Legendary]
                    } else {
                        vec![]
                    })
                    .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                    .build();
                    game.create_object_from_card(&card, owner, Zone::Battlefield);
                }
            }
            let mana_value = 2 * count + u32::from(over_limit);
            let spell =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast Candidate")
                    .card_types(vec![CardType::Instant])
                    .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                        crate::mana::ManaSymbol::Generic(mana_value as u8),
                    ]))
                    .parse_text("You gain 3 life.")
                    .unwrap();
            let card = game.create_object_from_definition(&spell, alice, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::BeginningOfCombatEvent::new(alice),
                crate::provenance::ProvNodeId::default(),
            );
            let triggers = crate::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptCast).unwrap();
            assert_eq!(
                game.stack.len(),
                usize::from(!over_limit),
                "count={count}, mana_value={mana_value}"
            );
            if !over_limit {
                assert_eq!(
                    game.object(game.stack[0].object_id).unwrap().stable_id,
                    stable
                );
                crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(alice).unwrap().life, 23);
            } else {
                assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
            }
        }
    }
}

#[test]
fn scaled_free_cast_renders_its_bound_limit() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Scaling Cast Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        TEXT
    );
}

#[test]
fn scaled_free_cast_counts_attackers_at_resolution() {
    let text = "Veil of Time — Whenever this creature attacks, you may cast a spell with mana value X or less from your hand without paying its mana cost, where X is the number of attacking creatures.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Epistolary Librarian")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 4))
            .parse_text(text)
            .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    for count in [1u32, 3] {
        for remove_attacker in [false, true] {
            for over_limit in [false, true] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let mut combat = crate::combat_state::CombatState::default();
                combat.attackers.push(crate::combat_state::AttackerInfo {
                    creature: source,
                    target: crate::combat_state::AttackTarget::Player(bob),
                });
                let creature =
                    crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other Creature")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                        .build();
                for index in 0..5 {
                    let id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
                    if index < count - 1 {
                        combat.attackers.push(crate::combat_state::AttackerInfo {
                            creature: id,
                            target: crate::combat_state::AttackTarget::Player(bob),
                        });
                    }
                }
                game.combat = Some(combat);
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CreatureAttackedEvent::with_total_attackers(
                        source,
                        crate::events::AttackEventTarget::Player(bob),
                        count as usize,
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                assert_eq!(triggers.len(), 1);
                let mut queue = crate::triggers::TriggerQueue::new();
                for trigger in triggers {
                    queue.add(trigger);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if remove_attacker {
                    game.combat.as_mut().unwrap().attackers.pop();
                }
                let limit = count - u32::from(remove_attacker);
                let spell =
                    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast Candidate")
                        .card_types(vec![CardType::Sorcery])
                        .mana_cost(crate::mana::ManaCost::from_symbols(vec![
                            crate::mana::ManaSymbol::Generic((limit + u32::from(over_limit)) as u8),
                        ]))
                        .parse_text("You gain 3 life.")
                        .unwrap();
                let card = game.create_object_from_definition(&spell, alice, Zone::Hand);
                crate::game_loop::resolve_stack_entry_with(&mut game, &mut AcceptCast).unwrap();
                assert_eq!(
                    game.stack.len(),
                    usize::from(!over_limit),
                    "count={count}, remove={remove_attacker}, over={over_limit}"
                );
                if !over_limit {
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                    assert_eq!(game.player(alice).unwrap().life, 23);
                } else {
                    assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
                }
            }
        }
    }
}

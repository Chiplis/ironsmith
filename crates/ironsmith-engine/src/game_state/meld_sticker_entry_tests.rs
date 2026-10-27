use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::effects::{ExecutionContext, MeldEffect};
use crate::{Ability, CardId, CardType, Effect, StaticAbility};

#[test]
fn meld_stickers_survive_component_cleanup_and_keep_order_for_either_leading_card() {
    for lead in 0..2 {
        for reverse in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let alice = PlayerId::from_index(0);
            let definitions = ["Graf Rats", "Midnight Scavengers", "Chittering Host"].map(|name| {
                CardDefinitionBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 3))
                    .build()
            });
            for definition in &definitions {
                game.register_linked_face_definition(definition);
            }
            let cards = [
                game.create_object_from_definition(&definitions[0], alice, Zone::Battlefield),
                game.create_object_from_definition(&definitions[1], alice, Zone::Battlefield),
            ];
            let order = if reverse { [1, 0] } else { [0, 1] };
            for (placement, index) in order.into_iter().enumerate() {
                game.put_name_sticker_on_object(
                    cards[index],
                    if placement == 0 { "Earlier" } else { "Later" },
                );
                let pt = game.add_accessible_pt_sticker(
                    alice,
                    0,
                    6 + placement as i32,
                    8 + placement as i32,
                );
                game.apply_pt_sticker(cards[index], pt).unwrap();
                let ability = game.add_accessible_ability_sticker(
                    alice,
                    0,
                    vec![Ability::static_ability(StaticAbility::flying())],
                    "Flying",
                );
                game.apply_ability_sticker(cards[index], ability).unwrap();
            }
            let mut branch = game.clone();
            struct Choices;
            impl crate::decision::DecisionMaker for Choices {}
            let mut dm = Choices;
            let mut ctx = ExecutionContext::new(cards[lead], alice, &mut dm);
            crate::effects::execute_effect(
                &mut branch,
                &Effect::new(MeldEffect::new("Chittering Host")),
                &mut ctx,
            )
            .unwrap();
            assert_eq!(branch.battlefield.len(), 1);
            let host = branch.battlefield[0];
            assert_eq!(
                branch.sticker_count_on_object(
                    host,
                    crate::events::KeywordActionKind::Sticker,
                    None
                ),
                6
            );
            // Both cards receive new timestamps during the native exile step.
            // The leading card departs first, so the other card's stickers are
            // the later set when the combined permanent enters.
            let latest = usize::from(order[1] != lead);
            let earlier_name = if latest == 1 { "Earlier" } else { "Later" };
            let later_name = if latest == 1 { "Later" } else { "Earlier" };
            assert_eq!(
                branch.current_name(host).unwrap(),
                format!("{later_name} {earlier_name} Chittering Host")
            );
            assert_eq!(branch.current_power(host), Some(6 + latest as i32));
            assert_eq!(branch.current_toughness(host), Some(8 + latest as i32));
            assert!(branch.object_has_ability(host, &StaticAbility::flying()));
            assert!(branch.object(cards[0]).is_none() && branch.object(cards[1]).is_none());
            assert_eq!(
                game.battlefield.len(),
                2,
                "the original native branch keeps its component stickers"
            );
            for id in cards {
                assert_eq!(
                    game.sticker_count_on_object(
                        id,
                        crate::events::KeywordActionKind::Sticker,
                        None
                    ),
                    3
                );
            }
            branch.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn counterpart_sticker_is_visible_to_entry_choices_without_moving_its_original_occurrence() {
    for suspend in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let alice = PlayerId::from_index(0);
        let mut definitions = ["Graf Rats", "Midnight Scavengers", "Chittering Host"].map(|name| {
            CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 3))
                .build()
        });
        definitions[2].abilities.push(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ));
        for definition in &definitions {
            game.register_linked_face_definition(definition);
        }
        let lead = game.create_object_from_definition(&definitions[0], alice, Zone::Battlefield);
        let other = game.create_object_from_definition(&definitions[1], alice, Zone::Battlefield);
        let stable = game.object(other).unwrap().stable_id;
        let sticker = game.add_accessible_ability_sticker(
            alice,
            0,
            vec![Ability::static_ability(StaticAbility::flying())],
            "Flying",
        );
        game.apply_ability_sticker(other, sticker).unwrap();
        assert!(!game.object_has_ability(lead, &StaticAbility::flying()));
        struct Inspect {
            original: StableId,
            suspend: bool,
            pending: bool,
            seen: usize,
        }
        impl crate::decision::DecisionMaker for Inspect {
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
            fn decide_colors(
                &mut self,
                game: &GameState,
                ctx: &crate::decisions::context::ColorsContext,
            ) -> Vec<crate::Color> {
                self.seen += 1;
                let entry = ctx.source.unwrap();
                let original = game.find_object_by_stable_id(self.original).unwrap();
                assert_eq!(game.object(entry).unwrap().zone, Zone::Exile);
                assert_eq!(game.object(original).unwrap().zone, Zone::Exile);
                assert!(
                    game.object_has_ability(entry, &StaticAbility::flying()),
                    "the composite candidate includes its counterpart's sticker"
                );
                assert!(
                    game.object_has_ability(original, &StaticAbility::flying()),
                    "the original card keeps the same physical sticker during prospective inspection"
                );
                self.pending = self.suspend;
                vec![crate::Color::Blue]
            }
        }
        let mut dm = Inspect {
            original: stable,
            suspend,
            pending: false,
            seen: 0,
        };
        let mut ctx = ExecutionContext::new(lead, alice, &mut dm);
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(MeldEffect::new("Chittering Host")),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(dm.seen, 1);
        if suspend {
            assert!(dm.pending);
            assert_eq!(game.battlefield.len(), 2);
            assert!(!game.object_has_ability(lead, &StaticAbility::flying()));
            assert!(game.object_has_ability(other, &StaticAbility::flying()));
            assert_eq!(game.sticker_effects_on_object(other).len(), 1);
        } else {
            assert_eq!(game.battlefield.len(), 1);
            let host = game.battlefield[0];
            assert!(game.object_has_ability(host, &StaticAbility::flying()));
            assert_eq!(
                game.sticker_effects_on_object(host).len(),
                1,
                "projection must not create a duplicate sticker occurrence"
            );
            assert_eq!(game.chosen_color(host), Some(crate::Color::Blue));
        }
        game.validate_zone_consistency().unwrap();
    }
}

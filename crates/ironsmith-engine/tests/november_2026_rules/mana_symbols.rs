use super::{A, game, put, resolve_spell};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::{DecisionMaker, GameState, PlayerId, Target, Zone};

#[test]
fn chroma_uses_the_copied_creatures_mana_cost() {
    for (printed, copied, bonus) in [
        ("ornithopter", "divinity-of-pride", 5),
        ("divinity-of-pride", "ornithopter", 0),
    ] {
        let mut g = game();
        put(&mut g, "light-from-within", A, Zone::Battlefield);
        let recipient = put(&mut g, printed, A, Zone::Battlefield);
        let model = put(&mut g, copied, PlayerId(1), Zone::Battlefield);
        resolve_spell(
            &mut g,
            "mirrorweave",
            vec![Target::Object(model)],
            &mut SelectFirstDecisionMaker,
        );
        assert_eq!(
            g.current_power(recipient),
            g.current_power(model).map(|p| p + bonus)
        );
        assert_eq!(
            g.current_toughness(recipient),
            g.current_toughness(model).map(|t| t + bonus)
        );
    }
}

#[test]
fn a_color_word_change_changes_which_mana_symbols_an_anthem_counts() {
    struct WhiteToGreen;
    impl DecisionMaker for WhiteToGreen {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            let choice = match ctx.description.as_str() {
                "Choose the word to replace" => "white",
                "Choose the replacement word" => "green",
                other => panic!("unexpected choice {other}"),
            };
            vec![
                ctx.options
                    .iter()
                    .find(|o| o.legal && o.description.eq_ignore_ascii_case(choice))
                    .unwrap()
                    .index,
            ]
        }
    }

    let mut g = game();
    let enchantment = put(&mut g, "light-from-within", A, Zone::Battlefield);
    let spirit = put(&mut g, "divinity-of-pride", A, Zone::Battlefield);
    let dinosaur = put(&mut g, "colossal-dreadmaw", A, Zone::Battlefield);
    assert_eq!(
        (g.current_power(spirit), g.current_power(dinosaur)),
        (Some(9), Some(6))
    );
    resolve_spell(
        &mut g,
        "sleight-of-mind",
        vec![Target::Object(enchantment)],
        &mut WhiteToGreen,
    );
    assert_eq!(
        (g.current_power(spirit), g.current_power(dinosaur)),
        (Some(4), Some(8))
    );
}

#[test]
fn battlefield_cost_filters_agree_for_a_copy_and_its_model() {
    use ironsmith::{ChoiceCount, Color, EffectContext, ObjectFilter, Value};
    for (printed, copied) in [
        ("ornithopter", "divinity-of-pride"),
        ("divinity-of-pride", "ornithopter"),
        ("ornithopter", "phyrexian-metamorph"),
        ("ornithopter", "endless-one"),
        ("endless-one", "ornithopter"),
    ] {
        let mut g = game();
        let recipient = put(&mut g, printed, A, Zone::Battlefield);
        let model = put(&mut g, copied, PlayerId(1), Zone::Battlefield);
        for id in [recipient, model] {
            g.add_counters(id, ironsmith::CounterType::PlusOnePlusOne, 2);
        }
        resolve_spell(
            &mut g,
            "mirrorweave",
            vec![Target::Object(model)],
            &mut SelectFirstDecisionMaker,
        );
        let cost = g.object(model).unwrap().mana_cost_owned();
        let ctx = EffectContext::new_default(model, A);
        let results = |id| {
            let mut filters: [ObjectFilter; 6] =
                std::array::from_fn(|_| ObjectFilter::specific(id));
            filters[0].exact_mana_cost = cost.clone();
            filters[1].colored_mana_symbol_count = Some(ChoiceCount::at_least(1));
            filters[2].mana_symbol_count = Some((Color::White, ChoiceCount::at_least(1)));
            filters[3].has_x_in_cost = true;
            filters[4].no_x_in_cost = true;
            filters[5].has_phyrexian_mana_symbol = true;
            filters.map(|filter| {
                ironsmith::effects::resolve_value(&g, &Value::Count(filter), &ctx).unwrap()
            })
        };
        let expected = match copied {
            "divinity-of-pride" => [1, 1, 1, 0, 1, 0],
            "phyrexian-metamorph" => [1, 1, 0, 0, 1, 1],
            "endless-one" => [1, 0, 0, 1, 0, 0],
            _ => [1, 0, 0, 0, 1, 0],
        };
        assert_eq!(results(model), expected);
        assert_eq!(results(recipient), expected, "{printed} copying {copied}");
    }
}

const DOLLMAKERS_SHOP: &str =
    include_str!("../../../../web/ui/public/cards/dollmaker-s-shop-porcelain-gallery.json");
const MIRROR_ROOM: &str =
    include_str!("../../../../web/ui/public/cards/mirror-room-fractured-realm.json");

fn enter_room(g: &mut GameState, fixture: &str) -> ironsmith::ObjectId {
    let json: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let mut halves: Vec<_> = json["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|half| {
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(
                &serde_json::from_value(half.clone()).unwrap(),
            )
            .unwrap()
        })
        .collect();
    let ids = [ironsmith::CardId::new(), ironsmith::CardId::new()];
    assert_eq!(halves.len(), 2);
    for (i, half) in halves.iter_mut().enumerate() {
        half.card.id = ids[i];
        half.card.other_face = Some(ids[1 - i]);
        g.register_linked_face_definition(half);
    }
    let id = g.create_object_from_definition(&halves[0], A, Zone::Hand);
    let stable = g.object(id).unwrap().stable_id;
    g.move_object_with_etb_processing_with_dm(id, Zone::Battlefield, &mut SelectFirstDecisionMaker)
        .unwrap();
    let room = g.find_object_by_stable_id(stable).unwrap();
    assert!(g.room_has_no_unlocked_door(room));
    g.add_counters(room, ironsmith::CounterType::PlusOnePlusOne, 1);
    room
}

#[test]
fn a_locked_room_copying_a_creature_has_its_name_cost_color_and_abilities() {
    use ironsmith::{
        EffectContext, ObjectFilter, Value, filter::Comparison, static_abilities::StaticAbilityId,
    };
    for fixture in [DOLLMAKERS_SHOP, MIRROR_ROOM] {
        let mut g = game();
        let room = enter_room(&mut g, fixture);
        put(&mut g, "opalescence", A, Zone::Battlefield);
        let model = put(&mut g, "divinity-of-pride", PlayerId(1), Zone::Battlefield);
        resolve_spell(
            &mut g,
            "mirrorweave",
            vec![Target::Object(model)],
            &mut SelectFirstDecisionMaker,
        );
        let actual = g.current_characteristics(room).unwrap();
        let original = g.current_characteristics(model).unwrap();
        assert_eq!(actual.name, original.name);
        assert_eq!(actual.mana_cost, original.mana_cost);
        assert_eq!(actual.colors, original.colors);
        assert_eq!(actual.abilities.len(), original.abilities.len());
        assert!(g.current_has_static_ability_id(room, StaticAbilityId::Flying));
        assert_eq!(actual.power, Some(5));
        let mut filter = ObjectFilter::specific(room);
        filter.mana_value = Some(Comparison::Equal(5));
        assert_eq!(
            ironsmith::effects::resolve_value(
                &g,
                &Value::Count(filter),
                &EffectContext::new_default(model, A)
            )
            .unwrap(),
            1
        );
        assert!(ironsmith::special_actions::locked_room_doors(&g, room).is_empty());
        g.effect_store.continuous_effects.cleanup_end_of_turn();
        g.refresh_continuous_state().unwrap();
        let original_room = g.current_characteristics(room).unwrap();
        assert!(original_room.name.is_empty() && original_room.mana_cost.is_none());
    }
}

#[test]
fn rooms_copy_both_halves_and_keep_the_recipients_left_and_right_designations() {
    use ironsmith::ManaSymbol;
    use ironsmith::special_actions::{
        self,
        RoomDoor::{Current, Linked},
        SpecialAction,
    };
    let mut dm = SelectFirstDecisionMaker;
    for initial in 0..4 {
        for donor_door in [Current, Linked] {
            let mut g = game();
            for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Colorless] {
                g.player_mut(A).unwrap().mana_pool.add(color, 50);
            }
            let room = enter_room(&mut g, DOLLMAKERS_SHOP);
            let model = enter_room(&mut g, MIRROR_ROOM);
            let unlock = |g: &mut GameState, room_id, door| {
                special_actions::perform(
                    SpecialAction::UnlockRoomDoor { room_id, door },
                    g,
                    A,
                    &mut SelectFirstDecisionMaker,
                )
                .unwrap();
            };
            for (bit, door) in [(1, Current), (2, Linked)] {
                if initial & bit != 0 {
                    unlock(&mut g, room, door);
                }
            }
            unlock(&mut g, model, donor_door);
            for door in special_actions::locked_room_doors(&g, model) {
                assert_eq!(
                    special_actions::room_door_is_right(&g, model, door),
                    donor_door == Current
                );
            }
            put(&mut g, "opalescence", PlayerId(1), Zone::Battlefield);
            resolve_spell(&mut g, "mirrorweave", vec![Target::Object(model)], &mut dm);
            let names = [
                "",
                "Mirror Room",
                "Fractured Realm",
                "Mirror Room // Fractured Realm",
            ];
            let costs = [None, Some(3), Some(7), Some(10)];
            let current = g.current_characteristics(room).unwrap();
            assert_eq!(current.name.as_ref(), names[initial]);
            assert_eq!(
                current.mana_cost.as_ref().map(|c| c.mana_value()),
                costs[initial]
            );
            for (bit, door, name, cost) in [
                (1, Current, "Mirror Room", "{2}{U}"),
                (2, Linked, "Fractured Realm", "{5}{U}{U}"),
            ] {
                assert_eq!(
                    special_actions::room_door_is_right(&g, room, door),
                    bit == 2
                );
                assert_eq!(
                    special_actions::room_door_name(&g, room, door).as_deref(),
                    Some(name)
                );
                assert_eq!(
                    special_actions::room_unlock_cost_display(&g, room, door).as_deref(),
                    (initial & bit == 0).then_some(cost)
                );
            }
            if initial & 2 != 0 {
                use ironsmith::{
                    EffectContext, EffectExecutor, ObjectFilter, PlayerFilter,
                    effects::UnlockRoomDoorEffect,
                };
                struct LockRight(ironsmith::ObjectId, String);
                impl DecisionMaker for LockRight {
                    fn decide_options(
                        &mut self,
                        _: &GameState,
                        ctx: &SelectOptionsContext,
                    ) -> Vec<usize> {
                        let option = if ctx.options.iter().any(|o| o.object_id == Some(self.0)) {
                            let room = ctx
                                .options
                                .iter()
                                .find(|o| o.object_id == Some(self.0))
                                .unwrap();
                            assert_eq!(room.description, self.1);
                            room
                        } else {
                            ctx.options
                                .iter()
                                .find(|o| o.description == "Lock Fractured Realm")
                                .unwrap()
                        };
                        vec![option.index]
                    }
                }
                let mut locked = g.clone();
                let mut choice = LockRight(room, names[initial].to_owned());
                let mut ctx = EffectContext::new_default(model, A).with_decision_maker(&mut choice);
                let effect = UnlockRoomDoorEffect::new(PlayerFilter::You, ObjectFilter::default())
                    .with_allow_lock(true);
                effect.execute(&mut locked, &mut ctx).unwrap();
                assert_eq!(
                    locked.current_name(room).as_deref(),
                    Some(names[initial & 1])
                );
                locked.effect_store.continuous_effects.cleanup_end_of_turn();
                locked.refresh_continuous_state().unwrap();
                assert_eq!(
                    locked.current_name(room).as_deref(),
                    Some(if initial == 2 { "" } else { "Dollmaker's Shop" })
                );
            }
            // Unlocking a copied right door pays the donor's blue cost. When
            // Mirrorweave expires the original right door stays unlocked.
            if initial & 2 == 0 {
                let blue = g.player(A).unwrap().mana_pool.blue;
                unlock(&mut g, room, Linked);
                assert!(blue - g.player(A).unwrap().mana_pool.blue >= 2);
            }
            g.effect_store.continuous_effects.cleanup_end_of_turn();
            g.refresh_continuous_state().unwrap();
            let name = g.current_name(room).unwrap();
            assert!(name.contains("Porcelain Gallery"));
            assert_eq!(name.contains("Dollmaker's Shop"), initial & 1 != 0);
        }
    }
}

#[test]
fn a_room_token_can_switch_doors_and_still_donate_both_halves() {
    use ironsmith::special_actions::{
        self,
        RoomDoor::{Current, Linked},
        SpecialAction,
    };
    use ironsmith::{ChooseSpec, Effect, ManaSymbol, StackEntry, effects::CreateTokenCopyEffect};

    for printed_door in [Current, Linked] {
        for recipient_door in [Current, Linked] {
            let mut g = game();
            let mut dm = SelectFirstDecisionMaker;
            for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Colorless] {
                g.player_mut(A).unwrap().mana_pool.add(color, 50);
            }
            let printed = enter_room(&mut g, MIRROR_ROOM);
            let recipient = enter_room(&mut g, DOLLMAKERS_SHOP);
            for (room_id, door) in [(printed, printed_door), (recipient, recipient_door)] {
                special_actions::perform(
                    SpecialAction::UnlockRoomDoor { room_id, door },
                    &mut g,
                    A,
                    &mut dm,
                )
                .unwrap();
            }
            let before = g.battlefield.clone();
            g.push_to_stack(StackEntry::ability(
                printed,
                A,
                vec![Effect::new(CreateTokenCopyEffect::one(
                    ChooseSpec::SpecificObject(printed),
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            let token = *g
                .battlefield
                .iter()
                .find(|id| !before.contains(id))
                .unwrap();
            // The token opens the half its model had locked.
            special_actions::perform(
                SpecialAction::UnlockRoomDoor {
                    room_id: token,
                    door: Linked,
                },
                &mut g,
                A,
                &mut dm,
            )
            .unwrap();
            put(&mut g, "opalescence", PlayerId(1), Zone::Battlefield);
            resolve_spell(&mut g, "mirrorweave", vec![Target::Object(token)], &mut dm);
            let (open, closed) = if recipient_door == Current {
                ("Mirror Room", "Fractured Realm")
            } else {
                ("Fractured Realm", "Mirror Room")
            };
            assert_eq!(g.current_name(recipient).as_deref(), Some(open));
            let locked = special_actions::locked_room_doors(&g, recipient);
            assert_eq!(locked.len(), 1);
            assert_eq!(
                special_actions::room_door_name(&g, recipient, locked[0]).as_deref(),
                Some(closed)
            );
        }
    }
}

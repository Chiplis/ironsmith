//! Frozen atlas observation 24743856: inspect dice table and target binding.
#[test]
fn aberrant_mind_canonical_structure() {
    let payloads = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(), "Aberrant Mind Sorcerer",
    ).unwrap();
    assert_eq!(payloads.len(), 1);
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    println!("{definition:#?}");
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payloads[0]);
    println!("{snapshot:#?}");
    assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled);
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
}

#[test]
fn aberrant_mind_die_boundaries_keep_exact_graveyard_target() {
    use ironsmith::{GameState, PlayerId, Zone, CardType, ObjectId};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::game_state::Target;
    struct Pick { target: ObjectId, excluded: Vec<ObjectId>, accept: bool, prompts: usize }
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
            assert_eq!(ctx.requirements.len(), 1);
            let legal = &ctx.requirements[0].legal_targets;
            assert!(legal.contains(&Target::Object(self.target)));
            for id in &self.excluded { assert!(!legal.contains(&Target::Object(*id))); }
            vec![Target::Object(self.target)]
        }
        fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.prompts += 1;
            self.accept
        }
    }
    let payloads = ironsmith_tools::load_card_payloads_by_name(ironsmith_tools::default_cards_path().to_str().unwrap(), "Aberrant Mind Sorcerer").unwrap();
    let definition = ironsmith_tools::compile_definition_from_payload(&payloads[0]).unwrap();
    let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
    for (roll, accept, change) in [(1,true,0),(9,true,0),(1,false,0),(9,false,0),(10,false,0),(20,false,0),(20,false,1),(20,false,2),(20,false,3)] {
        for kind in [CardType::Instant, CardType::Sorcery] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let spell = CardDefinitionBuilder::new(CardId::new(), "Target fixture").card_types(vec![kind]).build();
            let target = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
            let stable = game.object(target).unwrap().stable_id;
            let other = game.create_object_from_definition(&spell, alice, Zone::Graveyard);
            let enemy = game.create_object_from_definition(&spell, bob, Zone::Graveyard);
            let hand_card = game.create_object_from_definition(&spell, alice, Zone::Hand);
            let creature = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
            let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let source = game.move_object_with_etb_processing(hand, Zone::Battlefield).unwrap().new_id;
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for event in game.take_pending_trigger_events() {
                for entry in ironsmith::triggers::check_triggers(&game, &event) { queue.add(entry); }
            }
            assert_eq!(queue.entries.len(), 1);
            let mut dm = Pick { target, excluded: vec![enemy,hand_card,creature], accept, prompts: 0 };
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            if change == 1 { game.move_object_by_effect(source, Zone::Graveyard); }
            if change >= 2 {
                let departed = game.move_object_by_effect(target, Zone::Exile).unwrap();
                if change == 3 { game.move_object_by_effect(departed, Zone::Graveyard); }
            }
            game.force_next_die_roll(roll);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let moved = game.find_object_by_stable_id(stable).unwrap();
            let expected = if change == 2 { Zone::Exile } else if change == 3 { Zone::Graveyard } else if roll >= 10 { Zone::Hand } else if accept { Zone::Library } else { Zone::Graveyard };
            assert_eq!(game.object(moved).unwrap().zone, expected, "roll={roll}, accept={accept}");
            if expected == Zone::Library { assert_eq!(game.player(alice).unwrap().library.last(), Some(&moved)); }
            assert_eq!(game.object(other).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.object(enemy).unwrap().zone, Zone::Graveyard);
            assert_eq!(dm.prompts, usize::from(roll <= 9 && change < 2));
            if change >= 2 {
                assert!(game.turn_store.turn_history.die_rolls_this_turn.get(&alice).is_none());
                assert_eq!(game.take_forced_die_roll(), Some(roll), "illegal target must stop before rolling");
            } else {
                assert_eq!(game.turn_store.turn_history.die_rolls_this_turn.get(&alice), Some(&vec![roll]));
            }
        }
    }
}

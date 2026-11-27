use super::{A, game, put, resolve_spell};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::decisions::context::{NumberContext, SelectOptionsContext};
use ironsmith::game_state::HiddenCardInfo;
use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod, perform};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{DecisionMaker, GameResult, GameState, ManaSymbol, ObjectId, PlayerId, Zone};

const B: PlayerId = PlayerId(1);

fn reveals(game: &GameState) -> Vec<(PlayerId, String)> {
    let mut revealed: Vec<_> = game
        .ui_effect_events()
        .filter(|event| event.kind == "reveal")
        .map(|event| (event.player.unwrap(), event.text.clone().unwrap()))
        .collect();
    revealed.sort();
    revealed
}

#[derive(Default)]
struct ChooseTwo(usize);
impl DecisionMaker for ChooseTwo {
    fn decide_number(&mut self, _: &GameState, c: &NumberContext) -> u32 {
        self.0 += 1;
        c.max.min(2)
    }
}

#[test]
fn manifest_and_cloak_pay_the_printed_cost_with_x_zero() {
    for (route, cost) in [("walking-ballista", 0), ("hooded-hydra", 2)] {
        for cloak in [false, true] {
            let mut g = game();
            let id = put(&mut g, route, A, Zone::Battlefield);
            g.set_face_down(id);
            if cloak {
                g.set_cloaked(id);
            } else {
                g.set_manifested(id);
            }
            g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 8);
            let mut dm = ChooseTwo::default();
            perform(
                SpecialAction::TurnFaceUp {
                    permanent_id: id,
                    method: TurnFaceUpMethod::PrintedManaCost,
                },
                &mut g,
                A,
                &mut dm,
            )
            .unwrap();
            assert!(!g.is_face_down(id));
            assert_eq!(dm.0, 0);
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 8 - cost);
            if route == "hooded-hydra" {
                assert_eq!(
                    g.object(id)
                        .unwrap()
                        .counters
                        .get(&ironsmith::CounterType::PlusOnePlusOne),
                    Some(&5)
                );
            }
        }
    }
}

#[test]
fn a_manifested_morph_still_has_an_independent_variable_morph_cost() {
    let mut g = game();
    let id = put(&mut g, "warbreak-trumpeter", A, Zone::Battlefield);
    g.set_face_down(id);
    g.set_manifested(id);
    g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 5);
    let mut dm = ChooseTwo::default();
    perform(
        SpecialAction::TurnFaceUp {
            permanent_id: id,
            method: TurnFaceUpMethod::TurnFaceUpAbility,
        },
        &mut g,
        A,
        &mut dm,
    )
    .unwrap();
    assert_eq!(dm.0, 1);
    assert_eq!(g.object(id).unwrap().x_value, Some(2));
    assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
}

fn merge_face_down_host(g: &mut GameState, host: ObjectId, mutator_on_top: bool) {
    let owner = g.object(host).unwrap().owner;
    g.set_face_down(host);
    let mutator = put(g, "cloudpiercer", owner, Zone::Stack);
    assert_eq!(
        g.merge_mutating_creature_spell(mutator, host, mutator_on_top),
        Some(host)
    );
}

#[test]
fn every_face_down_component_is_revealed_at_each_required_boundary() {
    let mut original = game();
    let id = put(&mut original, "grizzly-bears", A, Zone::Battlefield);
    for mutator_on_top in [false, true] {
        let mut merged = original.clone();
        merge_face_down_host(&mut merged, id, mutator_on_top);
        for turn_both_down in [false, true] {
            let mut board = merged.clone();
            if turn_both_down {
                board.set_face_down(id);
            }
            for boundary in ["hand", "library", "graveyard", "exile", "leave", "end"] {
                let mut g = board.clone();
                match boundary {
                    "leave" => {
                        assert!(g.leave_game(A).unwrap());
                    }
                    "end" => {
                        g.player_mut(B).unwrap().has_won = true;
                        ironsmith::game_loop::advance_priority(&mut g, &mut TriggerQueue::new())
                            .unwrap();
                    }
                    _ => {
                        let zone = match boundary {
                            "hand" => Zone::Hand,
                            "library" => Zone::Library,
                            "graveyard" => Zone::Graveyard,
                            _ => Zone::Exile,
                        };
                        g.move_object_by_effect(id, zone).unwrap();
                    }
                }
                let mut expected = vec![(A, "Grizzly Bears".to_string())];
                if mutator_on_top && turn_both_down {
                    expected.insert(0, (A, "Cloudpiercer".into()));
                }
                assert_eq!(
                    reveals(&g),
                    expected,
                    "{boundary} {mutator_on_top} {turn_both_down}"
                );
            }
        }
    }
}

#[test]
fn trying_to_turn_up_a_merged_instant_reveals_the_cards_and_leaves_it_face_down() {
    let mut g = game();
    let id = put(&mut g, "counterspell", A, Zone::Battlefield);
    merge_face_down_host(&mut g, id, true);
    assert!(g.set_face_down(id));
    g.set_current_controller(id, B).unwrap();
    assert!(!g.set_face_up(id).unwrap());
    assert!(g.is_face_down(id));
    assert_eq!(
        reveals(&g),
        vec![(B, "Cloudpiercer".into()), (B, "Counterspell".into())]
    );
}

struct CopyBears;
impl DecisionMaker for CopyBears {
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        c.options
            .iter()
            .find(|o| o.legal && o.description.contains("Grizzly Bears"))
            .map(|o| vec![o.index])
            .unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(g, c))
    }
}

#[test]
fn a_clone_turned_face_down_reveals_and_returns_as_the_physical_clone() {
    for merged in [false, true] {
        let mut g = game();
        let donor = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        resolve_spell(&mut g, "clone", vec![], &mut CopyBears);
        let copy = *g.battlefield.iter().find(|id| **id != donor).unwrap();
        assert_eq!(g.current_name(copy).as_deref(), Some("Grizzly Bears"));
        let stable = g.object(copy).unwrap().stable_id;
        if merged {
            merge_face_down_host(&mut g, copy, true);
        } else {
            g.set_face_down(copy);
        }
        g.move_object_by_effect(copy, Zone::Hand).unwrap();
        let returned = g.find_object_by_stable_id(stable).unwrap();
        assert_eq!(g.object(returned).unwrap().name, "Clone");
        assert_eq!(reveals(&g), vec![(A, "Clone".into())]);
    }
}

#[test]
fn leaving_reveals_only_the_departing_players_foretold_cards() {
    let mut g = game();
    for owner in [A, B] {
        let card = put(&mut g, "behold-the-multiverse", owner, Zone::Exile);
        g.set_face_down(card);
        g.set_foretold(card);
    }
    let other = put(&mut g, "counterspell", A, Zone::Exile);
    g.set_face_down(other);
    assert!(g.leave_game(B).unwrap());
    assert_eq!(reveals(&g), vec![(B, "Behold the Multiverse".into())]);
}

#[test]
fn a_restart_or_completed_subgame_discloses_cards_from_the_game_that_ended() {
    for subgame in [false, true] {
        let mut g = game();
        if subgame {
            g.begin_subgame(None, A, vec![]).unwrap();
            g.complete_subgame_starting_procedure();
        }
        let id = put(&mut g, "grizzly-bears", B, Zone::Battlefield);
        merge_face_down_host(&mut g, id, true);
        if subgame {
            g.finish_subgame_with(GameResult::Winner(A), &mut SelectFirstDecisionMaker)
                .unwrap();
        } else {
            g.restart_game(A, &[]);
        }
        assert_eq!(reveals(&g), vec![(B, "Grizzly Bears".into())]);
    }
}

#[test]
fn a_tracked_component_under_a_face_up_mutator_keeps_its_physical_identity() {
    for leave in [false, true] {
        let mut g = game();
        let id = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
        let stable = g.object(id).unwrap().stable_id;
        g.set_hidden_card_info(
            id,
            HiddenCardInfo {
                incarnation: Some(3),
                owner: A,
                zone: Zone::Battlefield,
                slot: 0,
                commitment: "bears".into(),
                origin_slot: None,
                origin_commitment: None,
                public_slot: None,
                public_commitment: None,
            },
        );
        merge_face_down_host(&mut g, id, true);
        if leave {
            assert!(g.leave_game(A).unwrap());
        }
        let openings = g.end_of_match_disclosure_cards(A);
        let opening = openings.iter().find(|entry| entry.object_id == id).unwrap();
        assert_eq!(opening.known_name.as_deref(), Some("Grizzly Bears"));
        assert!(opening.face_down);
        if !leave {
            g.move_object_by_effect(id, Zone::Hand).unwrap();
            let returned = g.find_object_by_stable_id(stable).unwrap();
            let info = g.hidden_card_info(returned).unwrap();
            assert_eq!(info.zone, Zone::Hand);
            assert_eq!(info.incarnation, Some(4));
            assert_eq!(info.commitment, "bears");
        }
    }
}

#[test]
fn cyber_conversion_defines_copiable_values_only_while_the_creature_is_face_down() {
    use ironsmith::{CardType, Subtype, Target};
    let mut g = game();
    let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    resolve_spell(
        &mut g,
        "cyber-conversion",
        vec![Target::Object(bear)],
        &mut dm,
    );
    let cyberman = g.calculated_copiable_values(bear).unwrap();
    assert!(cyberman.card_types.contains(&CardType::Artifact));
    assert!(cyberman.subtypes.contains(&Subtype::Cyberman));
    let opaque = g.create_hidden_card_placeholder(A, Zone::Battlefield, 0, "creature".into());
    g.set_face_down(opaque);
    g.object_mut(opaque)
        .unwrap()
        .apply_custom_face_down_overlay(cyberman);
    g.reveal_hidden_card_with_definition(opaque, &super::card("grizzly-bears"))
        .unwrap();
    assert!(
        g.calculated_copiable_values(opaque)
            .unwrap()
            .subtypes
            .contains(&Subtype::Cyberman)
    );
    g.set_face_up(bear).unwrap();
    assert!(!g.current_has_subtype(bear, Subtype::Cyberman));
    assert!(g.current_has_subtype(bear, Subtype::Bear));

    g.set_face_down(bear);
    resolve_spell(
        &mut g,
        "cyber-conversion",
        vec![Target::Object(bear)],
        &mut dm,
    );
    assert!(!g.current_has_subtype(bear, Subtype::Cyberman));
}

#[test]
fn essence_of_the_wild_copies_beneath_a_manifest_and_the_physical_card_sets_the_cost() {
    let mut original = game();
    put(&mut original, "essence-of-the-wild", A, Zone::Battlefield);
    for (route, can_pay) in [("grizzly-bears", true), ("counterspell", false)] {
        let mut g = original.clone();
        let card = put(&mut g, route, A, Zone::Library);
        let stable = g.object(card).unwrap().stable_id;
        let mut dm = SelectFirstDecisionMaker;
        resolve_spell(&mut g, "soul-summons", vec![], &mut dm);
        let permanent = g.find_object_by_stable_id(stable).unwrap();
        assert!(g.is_face_down(permanent));
        assert_eq!(g.current_power(permanent), Some(2));
        g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 2);
        let result = perform(
            SpecialAction::TurnFaceUp {
                permanent_id: permanent,
                method: TurnFaceUpMethod::PrintedManaCost,
            },
            &mut g,
            A,
            &mut dm,
        );
        if can_pay {
            result.unwrap();
            assert_eq!(
                g.current_name(permanent).as_deref(),
                Some("Essence of the Wild")
            );
            assert_eq!(g.current_power(permanent), Some(6));
            assert_eq!(g.player(A).unwrap().mana_pool.total(), 0);
            let bounced = g.move_object_by_effect(permanent, Zone::Hand).unwrap();
            assert_eq!(g.object(bounced).unwrap().name, "Grizzly Bears");
        } else {
            assert!(result.is_err());
            assert!(!g.set_face_up(permanent).unwrap());
            assert_eq!(reveals(&g), vec![(A, "Counterspell".into())]);
        }
    }
}

#[test]
fn yedora_returns_a_copiable_forest_and_its_face_up_creature_keeps_its_printed_types() {
    use ironsmith::decisions::context::BooleanContext;
    use ironsmith::{CardType, Subtype};
    struct Accept;
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
            c.can_accept
        }
    }
    let mut g = game();
    put(&mut g, "yedora-grave-gardener", A, Zone::Battlefield);
    let bear = put(&mut g, "grizzly-bears", A, Zone::Battlefield);
    let stable = g.object(bear).unwrap().stable_id;
    g.move_object_by_effect(bear, Zone::Graveyard).unwrap();
    let mut queue = TriggerQueue::new();
    let mut dm = Accept;
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
    assert!(!g.stack.is_empty());
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    let forest = g.find_object_by_stable_id(stable).unwrap();
    assert!(g.is_face_down(forest));
    let values = g.calculated_copiable_values(forest).unwrap();
    assert_eq!(values.card_types, vec![CardType::Land]);
    assert_eq!(values.subtypes, vec![Subtype::Forest]);
    assert_eq!(values.power, None);
    assert!(g.set_face_up(forest).unwrap());
    assert_eq!(
        g.current_card_types(forest).unwrap(),
        vec![CardType::Creature]
    );
    assert!(g.current_has_subtype(forest, Subtype::Bear));
}

#[test]
fn magars_two_quoted_abilities_are_part_of_its_face_down_creatures_copiable_values() {
    use ironsmith::{AbilityKind, StackEntry, Target};
    let mut g = game();
    let magar = put(&mut g, "magar-of-the-magic-strings", A, Zone::Battlefield);
    let instant = put(&mut g, "counterspell", A, Zone::Graveyard);
    let stable = g.object(instant).unwrap().stable_id;
    let program = g
        .object(magar)
        .unwrap()
        .abilities
        .iter()
        .find_map(|ability| {
            if let AbilityKind::Activated(a) = &ability.kind {
                Some(a.effects.clone())
            } else {
                None
            }
        })
        .unwrap();
    let mut entry = StackEntry::ability(magar, A, program);
    entry.targets.push(Target::Object(instant));
    g.push_to_stack(entry);
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
    let permanent = g.find_object_by_stable_id(stable).unwrap();
    assert!(g.is_face_down(permanent));
    let values = g.calculated_copiable_values(permanent).unwrap();
    assert_eq!((values.power, values.toughness), (Some(3), Some(3)));
    assert!(
        values
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
    );
    assert!(
        values
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Static(_)))
    );
}

#[test]
fn the_cyber_controller_defines_the_form_of_every_creature_returned_by_its_trigger() {
    use ironsmith::{CardType, StackEntry, Subtype};
    let mut g = game();
    let bear = put(&mut g, "grizzly-bears", B, Zone::Library);
    let stable = g.object(bear).unwrap().stable_id;
    let spell = put(&mut g, "the-cyber-controller", A, Zone::Stack);
    let mut entry = StackEntry::new(spell, A);
    g.object_mut(spell).unwrap().x_value = Some(1);
    entry.x_value = Some(1);
    g.push_to_stack(entry);
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut g, &mut queue).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut g, &mut queue, &mut dm).unwrap();
    ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
    let returned = g.find_object_by_stable_id(stable).unwrap();
    assert!(g.is_face_down(returned));
    let cyberman = g.calculated_copiable_values(returned).unwrap();
    assert!(cyberman.card_types.contains(&CardType::Artifact));
    assert!(cyberman.subtypes.contains(&Subtype::Cyberman));
    assert_eq!(cyberman.power, Some(2));
    assert_eq!(g.current_power(returned), Some(3));
}

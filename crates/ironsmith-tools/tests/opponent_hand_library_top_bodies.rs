//! Source-only frozen-body gates. UNRUN: no execution or measurement credit.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::{SelectObjectsContext, SelectionRevealPolicy, TargetsContext, ViewCardsContext};
use ironsmith::game_loop::{drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{AbilityKind, CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const NAMES: [&str; 2] = ["Chittering Rats", "Chimney Imp"];
#[derive(Clone, Copy, Debug)]
enum Route { Direct, Artifact }
const ROUTES: [Route; 2] = [Route::Direct, Route::Artifact];

fn compile(name: &str, text: &str, route: Route) -> CardDefinition {
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name);
    match route {
        Route::Direct => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_runtime_definition(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            result.unwrap()
        }
        Route::Artifact => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_artifact(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            let (artifact, _) = result.unwrap();
            artifact.validate().unwrap();
            let wire = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
            ironsmith::artifact_materializer::materialize_artifact(&wire).unwrap()
        }
    }
}
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/opponent_hand_library_top_bodies.json.fixture")).unwrap()
}
fn definition(name: &str, route: Route) -> CardDefinition {
    let rows = rows();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let metadata = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap());
    compile(name, &format!("{metadata}\n{}", row["oracle_text"].as_str().unwrap()), route)
}

#[test]
fn exact_complete_frozen_bodies_compile_independently_without_loss() {
    let rows = rows();
    assert_eq!(rows.len(), 2);
    for (name, oracle_id) in [(NAMES[0], "08dfe42e-35c0-4be0-abba-57269792ff3d"),
        (NAMES[1], "3901bf30-b7c1-4977-a7b1-fcdafcc266cd")] {
        assert!(rows.iter().any(|row| row["name"] == name && row["oracle_id"] == oracle_id));
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        let metadata_lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
            format!("Type: {}", row["type_line"].as_str().unwrap()),
            format!("Power/Toughness: {}/{}", row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap())];
        let oracle = row["oracle_text"].as_str().unwrap();
        let payload = ironsmith_tools::CardPayload {
            name: name.into(), parse_name: None, oracle_text: oracle.into(), raw_oracle_text: oracle.into(),
            parse_input: ironsmith_tools::build_parse_input(&metadata_lines, oracle), metadata_lines,
            other_face_name: None, linked_face_layout: None,
        };
        let snapshot = ironsmith_tools::compile_strict_snapshot_from_payload(&payload);
        assert_eq!(snapshot.parse_status, ironsmith_tools::ParseStatus::StrictCompiled);
        assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented && snapshot.parse_error.is_none());
        for route in ROUTES {
            let definition = definition(name, route);
            assert_eq!(definition.name(), name);
            assert_eq!(definition.card.card_types, vec![CardType::Creature]);
            use ironsmith::mana::{ManaCost, ManaSymbol};
            let cost = if name == NAMES[0] {
                ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)], vec![ManaSymbol::Black], vec![ManaSymbol::Black]])
            } else {
                ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)], vec![ManaSymbol::Black]])
            };
            assert_eq!(definition.card.mana_cost.as_ref(), Some(&cost));
            assert_eq!(definition.card.subtypes, vec![if name == NAMES[0] { ironsmith::Subtype::Rat } else { ironsmith::Subtype::Imp }]);
            assert_eq!(definition.abilities.len(), if name == NAMES[0] { 1 } else { 2 });
            let triggers: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Triggered(trigger) => Some(trigger), _ => None,
            }).collect();
            assert_eq!(triggers.len(), 1);
            assert_eq!(triggers[0].choices.len(), 1, "only the opponent is targeted, never a hidden card");
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert_eq!(game.calculated_power(source), Some(if name == NAMES[0] { 2 } else { 1 }));
            assert_eq!(game.calculated_toughness(source), Some(2));
            assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::Flying), name == NAMES[1]);
        }
    }
}

struct Pick {
    opponent: PlayerId,
    hand: Vec<ObjectId>,
    chosen: Option<ObjectId>,
    legal_opponents: Vec<PlayerId>,
    target_prompts: usize,
    card_prompts: usize,
    pause: bool,
    pending: bool,
}
impl DecisionMaker for Pick {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(ctx.player, A, "the trigger controller announces the opponent target");
        assert_eq!(ctx.requirements.len(), 1);
        let requirement = &ctx.requirements[0];
        assert_eq!(requirement.min_targets, 1);
        assert_eq!(requirement.max_targets, Some(1));
        let actual = &requirement.legal_targets;
        // Membership plus exact cardinality avoids relying on target ordering.
        assert_eq!(actual.len(), self.legal_opponents.len());
        for opponent in &self.legal_opponents { assert!(actual.contains(&Target::Player(*opponent))); }
        assert!(!actual.contains(&Target::Player(A)));
        self.target_prompts += 1;
        vec![Target::Player(self.opponent)]
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(ctx.player, self.opponent, "target opponent chooses, not source owner/controller");
        assert_eq!((ctx.min, ctx.max), (1, Some(1)));
        assert_ne!(ctx.reveal_policy, SelectionRevealPolicy::Public, "hand-to-library does not reveal");
        let mut actual: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        let mut expected = self.hand.clone(); actual.sort(); expected.sort();
        assert_eq!(actual, expected, "all and only targeted hand cards, including lands");
        self.card_prompts += 1;
        self.pending = self.pause;
        self.chosen.into_iter().collect()
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, _: &[ObjectId], ctx: &ViewCardsContext) {
        assert_eq!(viewer, self.opponent, "no disclosure to source controller or uninvolved opponent");
        assert!(!ctx.public);
    }
}
fn neutral(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, kind: CardType) -> ObjectId {
    let card = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![kind]).build();
    game.create_object_from_definition(&card, owner, zone)
}
fn enqueue(game: &mut GameState, dm: &mut Pick) {
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(game, &mut queue);
    assert_eq!(queue.entries.len(), 1, "real zone event must produce exactly the printed trigger");
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack.last().unwrap().controller, A);
}

#[test]
fn real_etb_and_death_choose_own_hidden_hand_top_and_preserve_other_seats() {
    // Empty, singleton, surplus, library-empty, source departure, post-target
    // hexproof, and native pending-choice rollback; both independent routes.
    for route in ROUTES { for name in NAMES { for opponent in [B, C] {
        for (count, empty_library, change, pause) in [
            (0, false, 0, false), (1, false, 0, false), (3, false, 0, false),
            (3, true, 0, false), (3, false, 1, false), (3, false, 2, false),
            (3, false, 0, true),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let mut hand = vec![];
            let mut unaffected = vec![];
            for seat in [A, B, C] {
                for i in 0..if seat == opponent { count } else { 2 } {
                    let id = neutral(&mut game, seat, Zone::Hand, &format!("Hand {seat:?} {i}"),
                        if i == 0 { CardType::Land } else { CardType::Instant });
                    game.set_hidden_card_info(id, ironsmith::game_state::HiddenCardInfo {
                        incarnation: Some(0), owner: seat, zone: Zone::Hand, slot: i as u16,
                        commitment: format!("library-top-{seat:?}-{i}"), origin_slot: None,
                        origin_commitment: None, public_slot: None, public_commitment: None,
                    });
                    if seat == opponent { hand.push(id); } else { unaffected.push(id); }
                }
                for zone in [Zone::Graveyard, Zone::Exile] {
                    unaffected.push(neutral(&mut game, seat, zone, "Wrong zone", CardType::Land));
                }
                if !(seat == opponent && empty_library) {
                    neutral(&mut game, seat, Zone::Library, "Library bottom", CardType::Land);
                    neutral(&mut game, seat, Zone::Library, "Library top", CardType::Instant);
                }
            }
            let old_libraries: Vec<_> = [A, B, C].iter().map(|seat| game.player(*seat).unwrap().library.clone()).collect();
            let old_zones: Vec<_> = unaffected.iter().map(|id| (*id, game.object(*id).unwrap().zone)).collect();
            let chosen = hand.last().copied(); // Surplus case deliberately chooses a non-first candidate.
            let stable = chosen.map(|id| game.object(id).unwrap().stable_id);
            let mut dm = Pick { opponent, hand: hand.clone(), chosen, legal_opponents: vec![B, C], target_prompts: 0,
                card_prompts: 0, pause, pending: false };
            let definition = definition(name, route);
            // Imp is owned by B and controlled by A at death; its owner remains a legal opponent target.
            let source = if name == NAMES[0] {
                let source = game.create_object_from_definition(&definition, A, Zone::Hand);
                let receipt = game.move_object_with_etb_processing(source, Zone::Battlefield).unwrap();
                assert!(!receipt.pending && receipt.programs.is_empty());
                receipt.original.into_result().unwrap().new_id
            } else {
                let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
                game.set_current_controller(source, A).unwrap();
                game.move_object_by_effect(source, Zone::Graveyard).unwrap()
            };
            enqueue(&mut game, &mut dm);
            assert_eq!(dm.target_prompts, 1);
            if change == 1 {
                if name == NAMES[0] { game.set_current_controller(source, opponent).unwrap(); }
                game.move_object_by_effect(source, Zone::Exile).unwrap();
            }
            if change == 2 {
                let defense = compile("Player defense", "Type: Enchantment\nYou have hexproof.", route);
                game.create_object_from_definition(&defense, opponent, Zone::Battlefield);
                game.refresh_continuous_state().unwrap();
            }
            let before_hands: Vec<_> = [A, B, C].iter().map(|seat| game.player(*seat).unwrap().hand.clone()).collect();
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            if pause {
                assert!(dm.pending);
                assert_eq!(game.stack.len(), 1, "pending resolution retains its native stack entry");
                for (i, seat) in [A, B, C].iter().enumerate() {
                    assert_eq!(game.player(*seat).unwrap().hand, before_hands[i]);
                    assert_eq!(game.player(*seat).unwrap().library, old_libraries[i]);
                }
                dm.pause = false; dm.pending = false;
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert!(game.stack.is_empty());
            let moved = count > 0 && change != 2;
            // An identity-free singleton is forced and needs no object decision.
            // The private view callback still checks its visibility boundary.
            assert_eq!(dm.card_prompts, if moved && count > 1 { if pause { 2 } else { 1 } } else { 0 });
            for (i, seat) in [A, B, C].iter().enumerate() {
                let mut library = old_libraries[i].clone();
                if *seat == opponent && moved {
                    let id = game.find_object_by_stable_id(stable.unwrap()).unwrap();
                    assert_eq!(game.object(id).unwrap().zone, Zone::Library);
                    assert_eq!(game.object(id).unwrap().owner, opponent);
                    assert!(!game.is_publicly_revealed_hidden_card(id));
                    library.push(id);
                }
                assert_eq!(game.player(*seat).unwrap().library, library, "top placement preserves entire prior order");
                assert_eq!(game.player(*seat).unwrap().hand.len(), before_hands[i].len() - usize::from(*seat == opponent && moved));
            }
            for (id, zone) in old_zones { assert_eq!(game.object(id).unwrap().zone, zone); }
            for id in hand.into_iter().filter(|id| Some(*id) != chosen || !moved) {
                assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
                assert!(!game.is_publicly_revealed_hidden_card(id));
            }
        }
    } } }
}

#[test]
fn unrelated_zone_changes_do_not_trigger_and_preexisting_player_hexproof_excludes_target() {
    for route in ROUTES { for name in NAMES {
        let definition = definition(name, route);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = neutral(&mut game, A, Zone::Hand, "Unrelated arrival", CardType::Land);
        let receipt = game.move_object_with_etb_processing(other, Zone::Battlefield).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        assert!(receipt.original.into_result().is_some());
        let mut queue = TriggerQueue::new();
        drain_pending_trigger_events(&mut game, &mut queue);
        assert!(queue.entries.is_empty(), "another object's entry does not trigger either body");
        game.move_object_by_effect(source, if name == NAMES[0] { Zone::Graveyard } else { Zone::Exile }).unwrap();
        drain_pending_trigger_events(&mut game, &mut queue);
        assert!(queue.entries.is_empty(), "Rats dying and Imp being exiled do not match their printed triggers");

        let defense = compile("Player defense", "Type: Enchantment\nYou have hexproof.", route);
        game.create_object_from_definition(&defense, C, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let receipt = game.move_object_with_etb_processing(hand, Zone::Battlefield).unwrap();
        assert!(!receipt.pending && receipt.programs.is_empty());
        let entered = receipt.original.into_result().unwrap().new_id;
        if name == NAMES[1] {
            drain_pending_trigger_events(&mut game, &mut queue);
            assert!(queue.entries.is_empty(), "Imp entering has no ETB trigger");
            game.move_object_by_effect(entered, Zone::Graveyard).unwrap();
        }
        let mut dm = Pick { opponent: B, hand: vec![], chosen: None, legal_opponents: vec![B],
            target_prompts: 0, card_prompts: 0, pause: false, pending: false };
        enqueue(&mut game, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.target_prompts, 1);
        assert_eq!(dm.card_prompts, 0);
        assert!(game.stack.is_empty());
    } }
}

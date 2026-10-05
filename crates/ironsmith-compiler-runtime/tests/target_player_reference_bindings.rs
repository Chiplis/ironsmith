//! UNVALIDATED: exact target declarations and their player/controller references.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/target_player_reference_bindings.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows = rows();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap(),
    ]
}
fn new_game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for player in [A, B, C] {
        for symbol in [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
            ManaSymbol::Colorless,
        ] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    game
}
fn printed(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    let id = game.create_object_from_definition(&definition, owner, zone);
    game.remove_summoning_sickness(id);
    id
}
fn creature(game: &mut GameState, owner: PlayerId, name: &str, mana: &str, power: i32) -> ObjectId {
    printed(
        game,
        owner,
        Zone::Battlefield,
        name,
        &format!("Mana cost: {mana}\nType: Creature — Bear\nPower/Toughness: {power}/8"),
    )
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, mana: &str) -> ObjectId {
    printed(
        game,
        owner,
        zone,
        name,
        &format!("Mana cost: {mana}\nType: Instant\nYou gain 1 life."),
    )
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    object_choosers: Vec<PlayerId>,
    object_pools: Vec<Vec<ObjectId>>,
    object_pool_owners: Vec<Vec<PlayerId>>,
    boolean_choosers: Vec<PlayerId>,
    accept: bool,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        for target in &self.targets {
            assert!(
                context
                    .requirements
                    .iter()
                    .any(|requirement| requirement.legal_targets.contains(target)),
                "{target:?}: {context:?}"
            );
        }
        self.targets.clone()
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_choosers.push(context.player);
        self.object_pool_owners.push(
            context
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .filter_map(|candidate| game.object(candidate.id).map(|object| object.owner))
                .collect(),
        );
        self.object_pools.push(
            context
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect(),
        );
        let selected: Vec<_> = self
            .objects
            .iter()
            .copied()
            .filter(|id| {
                context
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            })
            .collect();
        if !selected.is_empty() {
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.boolean_choosers.push(context.player);
        self.accept
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let mut state = PriorityLoopState::new(3);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() {
            return;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("announcement did not finish");
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==id))
        .expect("exact spell is castable");
    announce(game, action, dm);
    game.stack.last().unwrap().object_id
}
fn activate(
    game: &mut GameState,
    source: ObjectId,
    ordinal: usize,
    payer: PlayerId,
    dm: &mut Choices,
) {
    game.turn.priority_player = Some(payer);
    let index = game
        .current_abilities(source)
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_)))
        .nth(ordinal)
        .unwrap()
        .0;
    let action=compute_legal_actions(game,payer).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index} if *id==source && *ability_index==index)).unwrap();
    announce(game, action, dm);
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn named(game: &GameState, owner: PlayerId, zone: Zone, name: &str) -> bool {
    {
        let ids = match zone {
            Zone::Battlefield => &game.battlefield,
            Zone::Exile => &game.exile,
            Zone::Hand => &game.player(owner).unwrap().hand,
            Zone::Library => &game.player(owner).unwrap().library,
            Zone::Graveyard => &game.player(owner).unwrap().graveyard,
            _ => panic!("unsupported fixture zone"),
        };
        ids.iter()
            .filter_map(|id| game.object(*id))
            .any(|object| object.owner == owner && object.name.as_ref() == name)
    }
}
#[test]
fn exact_full_payloads_retain_artifact_bodies_without_promoting_two_partials() {
    let rows = rows();
    assert_eq!(rows.len(), 15);
    assert_eq!(
        rows.iter()
            .filter(|row| row["proposed_coverage"] == "partial_not_counted")
            .count(),
        2
    );
    for row in rows
        .iter()
        .filter(|row| row["proposed_coverage"] == "source_complete_unvalidated")
    {
        definitions(row["name"].as_str().unwrap());
    }
}
#[test]
fn target_subject_untaps_only_basic_lands_that_player_controls() {
    for definition in definitions("Early Harvest") {
        let mut game = new_game();
        let own = printed(
            &mut game,
            A,
            Zone::Battlefield,
            "Own",
            "Type: Basic Land — Forest",
        );
        let theirs = printed(
            &mut game,
            B,
            Zone::Battlefield,
            "Their",
            "Type: Basic Land — Forest",
        );
        let borrowed = printed(
            &mut game,
            C,
            Zone::Battlefield,
            "Borrowed",
            "Type: Basic Land — Island",
        );
        game.set_current_controller(borrowed, B).unwrap();
        let nonbasic = printed(
            &mut game,
            B,
            Zone::Battlefield,
            "Nonbasic",
            "Type: Land\n{T}: Add {G}.",
        );
        let third = printed(
            &mut game,
            C,
            Zone::Battlefield,
            "Third",
            "Type: Basic Land — Plains",
        );
        for id in [own, theirs, borrowed, nonbasic, third] {
            game.tap(id);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        for id in [theirs, borrowed] {
            assert!(!game.is_tapped(id));
        }
        for id in [own, nonbasic, third] {
            assert!(game.is_tapped(id));
        }
    }
}
#[test]
fn ember_gale_keeps_announced_player_even_when_the_first_affected_set_is_empty() {
    for definition in definitions("Ember Gale") {
        let mut game = new_game();
        let white = creature(&mut game, B, "White", "{W}", 2);
        let blue = creature(&mut game, B, "Blue", "{U}", 2);
        let both = creature(&mut game, B, "Both", "{W}{U}", 2);
        let red = creature(&mut game, B, "Red", "{R}", 2);
        let other = creature(&mut game, C, "Other", "{W}", 2);
        let attacker = creature(&mut game, A, "Attacker", "{G}", 2);
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        for id in [white, blue, both] {
            assert_eq!(game.damage_on(id), 1);
        }
        for id in [red, other, attacker] {
            assert_eq!(game.damage_on(id), 0);
        }
        assert!(!ironsmith::rules::combat::can_block(
            game.object(attacker).unwrap(),
            game.object(red).unwrap(),
            &game
        ));
        assert!(ironsmith::rules::combat::can_block(
            game.object(attacker).unwrap(),
            game.object(other).unwrap(),
            &game
        ));
        // Targeting an empty participant's set is still a real, singular player target.
        let mut empty = new_game();
        creature(&mut empty, C, "Unaffected", "{W}", 2);
        cast(&mut empty, &definition, &mut dm);
        resolve(&mut empty, &mut dm);
        assert_eq!(empty.player(C).unwrap().life, 20);
    }
}
#[test]
fn inquisition_counts_white_cards_in_the_revealed_players_hand_only() {
    for definition in definitions("Inquisition") {
        let mut game = new_game();
        for owner in [A, C] {
            for n in 0..4 {
                card(
                    &mut game,
                    owner,
                    Zone::Hand,
                    &format!("Other {owner:?} {n}"),
                    "{W}",
                );
            }
        }
        for (name, mana) in [
            ("White one", "{W}"),
            ("White two", "{W}"),
            ("Multicolor", "{W}{B}"),
            ("Blue", "{U}"),
        ] {
            card(&mut game, B, Zone::Hand, name, mana);
        }
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(B).unwrap().hand.len(), 4);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(C).unwrap().life, 20);
    }
}
#[test]
fn roiling_terrain_uses_departed_controller_not_owner_and_does_not_require_destruction() {
    for definition in definitions("Roiling Terrain") {
        for indestructible in [false, true] {
            let mut game = new_game();
            let land = printed(
                &mut game,
                A,
                Zone::Battlefield,
                "Borrowed land",
                if indestructible {
                    "Type: Land\nIndestructible"
                } else {
                    "Type: Land"
                },
            );
            game.set_current_controller(land, B).unwrap();
            for (player, n) in [(A, 4), (B, 2), (C, 6)] {
                for i in 0..n {
                    printed(
                        &mut game,
                        player,
                        Zone::Graveyard,
                        &format!("Old land {i}"),
                        "Type: Land",
                    );
                }
            }
            card(&mut game, B, Zone::Graveyard, "Nonland", "{B}");
            let mut dm = Choices {
                targets: vec![Target::Object(land)],
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(
                game.player(B).unwrap().life,
                18,
                "indestructible {indestructible}, life A {} B {} C {}",
                game.player(A).unwrap().life,
                game.player(B).unwrap().life,
                game.player(C).unwrap().life
            );
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(C).unwrap().life, 20);
            assert_eq!(game.object(land).is_some(), indestructible);
        }
    }
}
#[test]
fn dependent_damage_targets_use_the_exact_source_controller_and_exclude_the_source() {
    for name in ["Mutiny", "Breaking of the Fellowship"] {
        for definition in definitions(name) {
            let mut game = new_game();
            let source = creature(&mut game, B, "Damage source", "{R}", 3);
            let victim = creature(&mut game, B, "Recipient", "{G}", 2);
            let third = creature(&mut game, C, "Third player", "{G}", 2);
            let mut dm = Choices {
                targets: vec![Target::Object(source), Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.damage_on(source), 0);
            assert_eq!(game.damage_on(victim), 3);
            assert_eq!(game.damage_on(third), 0);
            assert_eq!(
                game.ring_temptations(A),
                u32::from(name == "Breaking of the Fellowship")
            );
            let mut impossible = new_game();
            creature(&mut impossible, B, "Lone B", "{R}", 3);
            creature(&mut impossible, C, "Lone C", "{G}", 2);
            let spell = impossible.create_object_from_definition(&definition, A, Zone::Hand);
            assert!(!compute_legal_actions(&impossible,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==spell)),"a different opponent or the source itself is not a legal second target");
        }
    }
}
#[test]
fn a_dependent_recipient_that_changes_control_is_illegal_but_the_remaining_clause_resolves() {
    for definition in definitions("Breaking of the Fellowship") {
        let mut game = new_game();
        let source = creature(&mut game, B, "Source", "{R}", 3);
        let victim = creature(&mut game, B, "Victim", "{G}", 2);
        let mut dm = Choices {
            targets: vec![Target::Object(source), Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        game.set_current_controller(victim, C).unwrap();
        resolve(&mut game, &mut dm);
        assert_eq!(game.damage_on(victim), 0);
        assert_eq!(game.ring_temptations(A), 1);
    }
}
#[test]
fn hand_replacement_search_keeps_caster_chooser_and_target_library_owner_distinct() {
    for name in ["Head Games", "Jester's Mask"] {
        for definition in definitions(name) {
            for hand_size in [0, 2] {
                let mut game = new_game();
                card(&mut game, A, Zone::Hand, "Own keep", "{W}");
                card(&mut game, C, Zone::Hand, "Third keep", "{U}");
                for n in 0..hand_size {
                    card(&mut game, B, Zone::Hand, &format!("Old {n}"), "{G}");
                }
                let first = card(&mut game, B, Zone::Library, "New one", "{R}");
                let second = card(&mut game, B, Zone::Library, "New two", "{B}");
                card(&mut game, A, Zone::Library, "Wrong library", "{W}");
                let mut dm = Choices {
                    targets: vec![Target::Player(B)],
                    objects: if hand_size == 0 {
                        vec![]
                    } else {
                        vec![first, second]
                    },
                    ..Default::default()
                };
                if name == "Jester's Mask" {
                    dm.targets.clear();
                    cast(&mut game, &definition, &mut dm);
                    resolve(&mut game, &mut dm);
                    let source = *game
                        .battlefield
                        .iter()
                        .find(|id| game.object(**id).unwrap().name.as_ref() == name)
                        .unwrap();
                    assert!(game.is_tapped(source));
                    game.untap(source);
                    dm.targets = vec![Target::Player(B)];
                    activate(&mut game, source, 0, A, &mut dm);
                    assert!(named(&game, A, Zone::Graveyard, name));
                } else {
                    cast(&mut game, &definition, &mut dm);
                }
                dm.object_choosers.clear();
                dm.object_pools.clear();
                dm.object_pool_owners.clear();
                resolve(&mut game, &mut dm);
                assert_eq!(game.player(B).unwrap().hand.len(), hand_size);
                if hand_size != 0 {
                    assert!(named(&game, B, Zone::Hand, "New one"));
                    assert!(named(&game, B, Zone::Hand, "New two"));
                    assert!(dm.object_choosers.iter().all(|player| *player == A));
                    assert!(
                        dm.object_pool_owners
                            .iter()
                            .flatten()
                            .all(|owner| *owner == B),
                        "the caster searches only the target participant's library"
                    );
                }
                assert!(named(&game, A, Zone::Hand, "Own keep"));
                assert!(named(&game, C, Zone::Hand, "Third keep"));
                let shuffles: Vec<_> = game
                    .turn_store
                    .turn_history
                    .event_records
                    .iter()
                    .chain(game.turn_store.turn_history.staged_event_records.iter())
                    .filter_map(|record| {
                        record
                            .event
                            .downcast::<ironsmith::events::ShuffleLibraryEvent>()
                    })
                    .map(|event| event.player)
                    .collect();
                assert_eq!(shuffles, vec![B]);
            }
        }
    }
}
#[test]
fn cellar_door_moves_only_the_bottom_of_the_target_library_and_creates_for_the_activator() {
    for definition in definitions("Cellar Door") {
        for creature_bottom in [false, true] {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let bottom = if creature_bottom {
                printed(
                    &mut game,
                    B,
                    Zone::Library,
                    "Bottom creature",
                    "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2",
                )
            } else {
                card(&mut game, B, Zone::Library, "Bottom instant", "{U}")
            };
            let top = card(&mut game, B, Zone::Library, "Top instant", "{U}");
            card(&mut game, C, Zone::Library, "Wrong bottom", "{R}");
            assert_eq!(game.player(B).unwrap().library[0], bottom);
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                ..Default::default()
            };
            let mana = game.player(A).unwrap().mana_pool.total();
            activate(&mut game, source, 0, A, &mut dm);
            assert!(game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 3);
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().library, vec![top]);
            assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
            assert_eq!(game.player(C).unwrap().library.len(), 1);
            let zombies: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| {
                    matches!(object.kind, ironsmith::object::ObjectKind::Token)
                        && object.subtypes.contains(&ironsmith::types::Subtype::Zombie)
                })
                .collect();
            assert_eq!(zombies.len(), usize::from(creature_bottom));
            assert!(
                zombies
                    .iter()
                    .all(|object| game.current_controller(object.id) == Some(A))
            );
        }
    }
}
#[test]
fn isildur_retains_four_as_the_comparison_boundary_and_the_destroyed_creatures_controller() {
    for definition in definitions("Isildur's Fateful Strike") {
        for hand_size in [4, 6] {
            let mut game = new_game();
            printed(
                &mut game,
                A,
                Zone::Battlefield,
                "Legend",
                "Type: Legendary Creature — Bear\nPower/Toughness: 1/8",
            );
            let victim = creature(&mut game, C, "Borrowed creature", "{G}", 2);
            game.set_current_controller(victim, B).unwrap();
            let cards: Vec<_> = (0..hand_size)
                .map(|n| card(&mut game, B, Zone::Hand, &format!("Victim hand {n}"), "{U}"))
                .collect();
            for n in 0..7 {
                card(&mut game, C, Zone::Hand, &format!("Owner hand {n}"), "{R}");
            }
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                objects: cards.iter().take(hand_size - 4).copied().collect(),
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert!(named(&game, C, Zone::Graveyard, "Borrowed creature"));
            assert_eq!(game.player(B).unwrap().hand.len(), 4);
            assert_eq!(game.player(C).unwrap().hand.len(), 7);
            assert!(dm.object_choosers.iter().all(|player| *player == B));
            assert_eq!(
                game.exile
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|object| object.owner == B)
                    .count(),
                hand_size - 4
            );
        }
    }
}
#[test]
fn meletis_gives_the_copy_to_the_target_spells_current_controller_and_fizzles_without_it() {
    for definition in definitions("Meletis Charlatan") {
        for remove_target in [false, true] {
            let mut game = new_game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let spell = card(&mut game, C, Zone::Stack, "Borrowed spell", "{U}");
            game.set_current_controller(spell, B).unwrap();
            game.stack.push(StackEntry::new(spell, B));
            let mut dm = Choices {
                targets: vec![Target::Object(spell)],
                ..Default::default()
            };
            activate(&mut game, source, 0, A, &mut dm);
            if remove_target {
                game.move_object(
                    spell,
                    Zone::Graveyard,
                    ironsmith::events::EventCause::effect(),
                )
                .unwrap();
            }
            resolve(&mut game, &mut dm);
            if remove_target {
                assert!(game.stack.is_empty());
            } else {
                assert_eq!(game.stack.len(), 2);
                assert!(game.stack.iter().all(|entry| entry.controller == B));
                assert_ne!(game.stack.last().unwrap().object_id, spell);
                assert!(dm.boolean_choosers.iter().all(|player| *player == B));
            }
        }
    }
}
#[test]
fn barroom_fight_and_optional_copy_belong_to_the_named_left_player() {
    for definition in definitions("Barroom Brawl") {
        for accept in [false, true] {
            let mut game = new_game();
            let own = creature(&mut game, A, "Own fighter", "{G}", 2);
            let other = creature(&mut game, B, "Left fighter", "{G}", 3);
            let mut dm = Choices {
                targets: vec![Target::Object(own), Target::Object(other)],
                accept,
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            // Declining retargeting is independent of accepting the copy. Set the
            // Boolean answer only for the copy prompt using a bounded custom DM.
            let mut resolving = CopyChoice {
                accept,
                asked: Vec::new(),
            };
            resolve_stack_entry_with(&mut game, &mut resolving).unwrap();
            assert_eq!(game.damage_on(own), 3);
            assert_eq!(game.damage_on(other), 2);
            assert!(!resolving.asked.is_empty());
            assert!(resolving.asked.iter().all(|player| *player == B));
            assert_eq!(game.stack.len(), usize::from(accept));
            if accept {
                assert_eq!(game.stack[0].controller, B);
            }
        }
    }
}
struct CopyChoice {
    accept: bool,
    asked: Vec<PlayerId>,
}
impl DecisionMaker for CopyChoice {
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.asked.push(context.player);
        self.accept
            && !context
                .description
                .to_ascii_lowercase()
                .contains("new target")
    }
}
#[test]
fn volraths_discard_cost_and_the_target_players_hand_choice_have_different_owners() {
    for definition in definitions("Volrath's Dungeon") {
        let mut game = new_game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let payment = card(&mut game, A, Zone::Hand, "Payment", "{G}");
        let remain = card(&mut game, A, Zone::Hand, "Own retained", "{G}");
        let chosen = card(&mut game, B, Zone::Hand, "Target choice", "{U}");
        let other = card(&mut game, B, Zone::Hand, "Target retained", "{U}");
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            objects: vec![payment],
            ..Default::default()
        };
        activate(&mut game, source, 1, A, &mut dm);
        assert!(named(&game, A, Zone::Graveyard, "Payment"));
        dm.objects = vec![chosen];
        dm.object_choosers.clear();
        resolve(&mut game, &mut dm);
        assert_eq!(dm.object_choosers, vec![B]);
        assert!(named(&game, B, Zone::Library, "Target choice"));
        assert_eq!(game.player(A).unwrap().hand, vec![remain]);
        assert_eq!(game.player(B).unwrap().hand, vec![other]);
        // The independent pay-life ability belongs to any player, only on
        // that player's turn. Its own source remains the enchantment.
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        let mut dm = Choices::default();
        activate(&mut game, source, 0, B, &mut dm);
        assert_eq!(game.player(B).unwrap().life, 15);
        resolve(&mut game, &mut dm);
        assert!(named(&game, A, Zone::Graveyard, "Volrath's Dungeon"));
    }
}
#[test]
fn orphan_player_references_remain_rejected_instead_of_defaulting_to_the_caster() {
    for text in [
        "Type: Sorcery\nUntap all lands that player controls.",
        "Type: Sorcery\nYou gain life equal to the number of cards in that player's hand.",
    ] {
        assert!(compile_to_runtime_definition("Missing antecedent", text, false).is_err());
    }
}
#[test]
fn all_illegal_damage_targets_fizzle_the_ring_clause_and_a_blinked_source_is_not_rebound() {
    for definition in definitions("Breaking of the Fellowship") {
        let mut game = new_game();
        let source = creature(&mut game, B, "Source", "{R}", 3);
        let victim = creature(&mut game, B, "Victim", "{G}", 2);
        let mut dm = Choices {
            targets: vec![Target::Object(source), Target::Object(victim)],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        let exiled = game
            .move_object(source, Zone::Exile, ironsmith::events::EventCause::effect())
            .unwrap();
        let returned = game
            .move_object(
                exiled,
                Zone::Battlefield,
                ironsmith::events::EventCause::effect(),
            )
            .unwrap();
        game.move_object(
            victim,
            Zone::Graveyard,
            ironsmith::events::EventCause::effect(),
        )
        .unwrap();
        assert_ne!(source, returned);
        resolve(&mut game, &mut dm);
        assert_eq!(game.ring_temptations(A), 0);
        assert_eq!(game.damage_on(returned), 0);
    }
}

#[test]
fn an_illegal_damage_source_does_not_use_lki_to_deal_damage_but_the_ring_still_tempts() {
    for definition in definitions("Breaking of the Fellowship") {
        for blink in [false, true] {
            let mut game = new_game();
            let source = creature(&mut game, B, "Departing damage source", "{R}", 3);
            let recipient = creature(&mut game, B, "Still legal recipient", "{G}", 2);
            let sibling = creature(&mut game, C, "Wrong participant", "{R}", 7);
            let mut dm = Choices {
                targets: vec![Target::Object(source), Target::Object(recipient)],
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            let departed = game
                .move_object(source, Zone::Exile, ironsmith::events::EventCause::effect())
                .unwrap();
            if blink {
                let returned = game
                    .move_object(
                        departed,
                        Zone::Battlefield,
                        ironsmith::events::EventCause::effect(),
                    )
                    .unwrap();
                assert_ne!(returned, source);
                game.set_current_controller(returned, C).unwrap();
            }
            resolve(&mut game, &mut dm);
            assert_eq!(game.damage_on(recipient), 0);
            assert_eq!(game.damage_on(sibling), 0);
            assert_eq!(
                game.ring_temptations(A),
                1,
                "one target remains legal, using the exact departed first target's controller"
            );
        }
    }
}

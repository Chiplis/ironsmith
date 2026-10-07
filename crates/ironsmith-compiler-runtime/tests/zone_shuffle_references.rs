//! UNVALIDATED full-card zone shuffle reference and participant regressions.
use std::collections::VecDeque;

use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
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
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/zone_shuffle_references.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!(
        "Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap()
    );
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = result.unwrap();
    let (result, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}
fn vanilla(
    game: &mut GameState,
    owner: PlayerId,
    zone: Zone,
    name: &str,
    creature: bool,
) -> ObjectId {
    let text = if creature {
        "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2"
    } else {
        "Mana cost: {1}\nType: Instant\nYou gain 1 life."
    };
    let (_, definition) = compile_to_artifact(name, text, false).unwrap();
    let id = game.create_object_from_definition(&definition, owner, zone);
    if zone == Zone::Stack {
        game.stack.push(StackEntry::new(id, owner));
    }
    id
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    object_choices: VecDeque<Vec<ObjectId>>,
    copy_targets: VecDeque<Vec<Target>>,
    retarget_choices: VecDeque<bool>,
    replicate_payments: usize,
    verify_stream_choices: bool,
    modes: Vec<usize>,
    expected_mode_max: Option<usize>,
    accept: bool,
    views: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
}
impl DecisionMaker for Choices {
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId],
        context: &ironsmith::decisions::context::ViewCardsContext) {
        self.views.push((viewer, context.subject, context.zone, context.public, cards.to_vec()));
    }
    fn decide_boolean(&mut self, game: &GameState, context: &BooleanContext) -> bool {
        if context.description.starts_with("Choose new targets for") {
            assert_eq!(context.player, A);
            assert_eq!(
                game.object(context.source.unwrap()).unwrap().kind,
                ironsmith::object::ObjectKind::SpellCopy
            );
            return self.retarget_choices.pop_front().unwrap();
        }
        self.accept
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.verify_stream_choices {
            assert_eq!(context.player, A);
            assert_eq!(context.min, 0);
            assert!(context.max.is_some_and(|max| max <= 4));
            for candidate in context.candidates.iter().filter(|candidate| candidate.legal) {
                let object = game.object(candidate.id).unwrap();
                assert_eq!(object.owner, A);
                assert_eq!(object.zone, Zone::Graveyard);
                assert_ne!(object.name.as_ref(), "Stream of Thought");
            }
        }
        let objects = self
            .object_choices
            .pop_front()
            .unwrap_or_else(|| self.objects.clone());
        assert!(objects.len() >= context.min);
        assert!(context.max.is_none_or(|max| objects.len() <= max));
        for id in &objects {
            assert!(
                context
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            );
        }
        objects
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs for") {
            if self.replicate_payments == 0 {
                return vec![];
            }
            let replicate = context
                .options
                .iter()
                .find(|option| option.legal && option.description.starts_with("Replicate:"))
                .expect("the real replicate payment must be offered");
            assert!(self.replicate_payments <= context.max);
            return vec![replicate.index; self.replicate_payments];
        }
        if context.description.starts_with("Choose mode for") {
            if let Some(max) = self.expected_mode_max {
                assert_eq!(context.max, max);
            }
            assert!(self.modes.len() >= context.min && self.modes.len() <= context.max);
            return self.modes.clone();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        if context.context == "copy" {
            assert_eq!(context.player, A);
            let targets = self.copy_targets.pop_front().unwrap();
            assert_eq!(context.requirements.len(), targets.len());
            for (requirement, target) in context.requirements.iter().zip(&targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            return targets;
        }
        self.targets
            .iter()
            .copied()
            .filter(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            })
            .collect()
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        choices,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)
            .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game.stack.iter().rev().find(|entry| !entry.is_ability).unwrap().object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    spell
}
fn resolve_one(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
    // Native resolution leaves entry and spell-copy events pending until the
    // next priority window. Preserve them so full-card triggers really run.
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..64 {
        if game.stack.is_empty() {
            return;
        }
        resolve_one(game, dm);
    }
    panic!("resolution did not settle");
}
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = LegalAction::ActivateAbility { source, ability_index };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm)
            .unwrap();
    }
    assert!(state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) {
    for index in 0..count {
        vanilla(
            game,
            owner,
            Zone::Library,
            &format!("Library {index}"),
            false,
        );
    }
}
fn shuffles(game: &GameState) -> Vec<PlayerId> {
    game.turn_store
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
        .collect()
}
#[test]
fn every_full_frozen_payload_round_trips_without_lost_body() {
    for name in [
        "Archangel's Light",
        "Mnemonic Nexus",
        "Molten Psyche",
        "Rite of Renewal",
        "Stream of Thought",
        "Whirlpool Warrior",
        "Winds of Change",
        "Visions",
    ] {
        definitions(name);
    }
}
#[test]
fn archangels_light_counts_graveyard_before_its_shuffle() {
    for definition in definitions("Archangel's Light") {
        let mut game = game();
        for _ in 0..3 {
            vanilla(&mut game, A, Zone::Graveyard, "Own grave", false);
        }
        vanilla(&mut game, B, Zone::Graveyard, "Foreign grave", false);
        library(&mut game, A, 4);
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 26);
        assert_eq!(game.player(A).unwrap().library.len(), 7);
        assert_eq!(
            game.player(A).unwrap().graveyard.len(),
            1,
            "only the resolving spell remains"
        );
        assert_eq!(game.player(B).unwrap().graveyard.len(), 1);
        assert_eq!(shuffles(&game), vec![A]);
    }
}
#[test]
fn mnemonic_nexus_keeps_each_graveyard_and_empty_participant_separate() {
    for definition in definitions("Mnemonic Nexus") {
        let mut game = game();
        for (player, count) in [(A, 2), (B, 3), (C, 0)] {
            library(&mut game, player, 5);
            for _ in 0..count {
                vanilla(&mut game, player, Zone::Graveyard, "Grave", false);
            }
        }
        let mut dm = Choices::default();
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(
            [
                game.player(A).unwrap().library.len(),
                game.player(B).unwrap().library.len(),
                game.player(C).unwrap().library.len()
            ],
            [7, 8, 5]
        );
        assert_eq!(shuffles(&game), vec![A, B, C]);
    }
}
#[test]
fn winds_and_molten_keep_actual_per_player_hand_and_draw_quantities() {
    for name in ["Winds of Change", "Molten Psyche"] {
        for definition in definitions(name) {
            let mut game = game();
            for (player, count) in [(A, 2), (B, 3), (C, 0)] {
                library(&mut game, player, 12);
                for _ in 0..count {
                    vanilla(&mut game, player, Zone::Hand, "Old hand", false);
                }
            }
            for _ in 0..3 {
                let artifact = ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Metalcraft")
                    .card_types(vec![ironsmith::CardType::Artifact])
                    .build();
                game.create_object_from_card(&artifact, A, Zone::Battlefield);
            }
            let mut dm = Choices::default();
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(
                [
                    game.player(A).unwrap().hand.len(),
                    game.player(B).unwrap().hand.len(),
                    game.player(C).unwrap().hand.len()
                ],
                [2, 3, 0]
            );
            assert_eq!(shuffles(&game), vec![A, B, C]);
            assert_eq!(
                game.player(B).unwrap().life,
                if name == "Molten Psyche" { 17 } else { 20 }
            );
            assert_eq!(game.player(C).unwrap().life, 20);
        }
    }
}
#[test]
fn stream_mills_the_target_but_selects_only_its_controllers_graveyard() {
    for definition in definitions("Stream of Thought") {
        let mut game = game();
        library(&mut game, A, 5);
        library(&mut game, B, 8);
        let selected: Vec<_> = (0..3)
            .map(|_| vanilla(&mut game, A, Zone::Graveyard, "Own grave", false))
            .collect();
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            objects: selected,
            verify_stream_choices: true,
            ..Default::default()
        };
        let mana = game.player(A).unwrap().mana_pool.total();
        cast(&mut game, &definition, &mut dm);
        assert_eq!(mana - game.player(A).unwrap().mana_pool.total(), 1);
        assert_eq!(game.stack.len(), 1, "declining replicate creates no copies");
        assert!(!game.stack[0].is_ability);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(B).unwrap().library.len(), 4);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 8);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(shuffles(&game), vec![A]);
    }
}
#[test]
fn stream_replicate_pays_twice_and_resolves_each_complete_body_with_its_own_choices() {
    for definition in definitions("Stream of Thought") {
        let mut game = game();
        library(&mut game, A, 5);
        library(&mut game, B, 12);
        library(&mut game, C, 12);
        let graves: Vec<_> = (0..5)
            .map(|index| {
                vanilla(&mut game, A, Zone::Graveyard, &format!("Own grave {index}"), false)
            })
            .collect();
        let foreign = vanilla(&mut game, B, Zone::Graveyard, "Foreign grave", false);
        let grave_identities: Vec<_> = graves
            .iter()
            .map(|id| game.object(*id).unwrap().stable_id)
            .collect();
        // The base {U} and two real {2}{U}{U} payments consume all nine mana.
        game.player_mut(A).unwrap().mana_pool.empty();
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 5);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 4);
        let mut dm = Choices {
            targets: vec![Target::Player(B)],
            replicate_payments: 2,
            retarget_choices: VecDeque::from([true, false]),
            copy_targets: VecDeque::from([vec![Target::Player(C)]]),
            object_choices: VecDeque::from([graves[..4].to_vec(), vec![], vec![graves[4]]]),
            verify_stream_choices: true,
            ..Default::default()
        };
        let spell = cast(&mut game, &definition, &mut dm);
        let spell_identity = game.object(spell).unwrap().stable_id;
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.stack.len(), 2, "one original and one replicate trigger");
        assert!(!game.stack[0].is_ability);
        assert_eq!(game.stack[0].object_id, spell);
        assert_eq!(game.stack[0].targets, vec![Target::Player(B)]);
        assert!(game.stack[1].is_ability);
        let paid = &game.stack[0].optional_costs_paid.costs;
        assert_eq!(paid.len(), 1);
        assert_eq!(paid[0].0.kind, ironsmith::cost::OptionalCostKind::Replicate);
        assert_eq!(paid[0].1, 2);
        assert!(shuffles(&game).is_empty(), "casting does not resolve the body");

        resolve_one(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 3, "exactly two copies, with no new cast triggers");
        assert!(game.stack.iter().all(|entry| !entry.is_ability && entry.controller == A));
        assert_eq!(game.stack[0].targets, vec![Target::Player(B)]);
        assert_eq!(game.stack[1].targets, vec![Target::Player(C)]);
        assert_eq!(game.stack[2].targets, vec![Target::Player(B)]);
        let copies: Vec<_> = game.stack[1..].iter().map(|entry| entry.object_id).collect();
        for copy in &copies {
            assert_eq!(
                game.object(*copy).unwrap().kind,
                ironsmith::object::ObjectKind::SpellCopy
            );
        }
        assert!(dm.retarget_choices.is_empty());
        assert!(dm.copy_targets.is_empty());
        assert!(shuffles(&game).is_empty());

        // The top copy keeps B and shuffles four controller-owned cards.
        resolve_one(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 2);
        assert_eq!(game.player(B).unwrap().library.len(), 8);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 5);
        assert_eq!(game.player(C).unwrap().library.len(), 12);
        assert_eq!(game.player(A).unwrap().library.len(), 9);
        assert_eq!(game.player(A).unwrap().graveyard, vec![graves[4]]);
        assert_eq!(shuffles(&game), vec![A]);
        assert!(game.object(copies[1]).is_none());

        // The retargeted copy mills C, and choosing zero still shuffles A.
        resolve_one(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(C).unwrap().library.len(), 8);
        assert_eq!(game.player(C).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 9);
        assert_eq!(game.player(A).unwrap().graveyard, vec![graves[4]]);
        assert_eq!(shuffles(&game), vec![A, A]);
        assert!(game.object(copies[0]).is_none());

        resolve_one(&mut game, &mut dm);
        assert!(game.stack.is_empty());
        assert!(dm.object_choices.is_empty());
        assert_eq!(game.player(B).unwrap().library.len(), 4);
        assert_eq!(game.player(B).unwrap().graveyard.len(), 9);
        assert_eq!(game.player(C).unwrap().library.len(), 8);
        assert_eq!(game.player(C).unwrap().graveyard.len(), 4);
        assert_eq!(game.player(A).unwrap().library.len(), 10);
        assert_eq!(shuffles(&game), vec![A, A, A]);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
        for stable in grave_identities {
            let card = game.find_object_by_stable_id(stable).unwrap();
            assert!(game.player(A).unwrap().library.contains(&card));
        }
        let finished_spell = game.find_object_by_stable_id(spell_identity).unwrap();
        assert_eq!(game.player(A).unwrap().graveyard, vec![finished_spell]);
        assert_eq!(
            game.turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter(|record| record.event.downcast::<ironsmith::events::SpellCastEvent>().is_some())
                .count(),
            1,
            "replicate copies were not cast"
        );
    }
}
#[test]
fn whirlpool_warrior_enters_then_pays_red_and_sacrifices_before_each_player_shuffles() {
    for definition in definitions("Whirlpool Warrior") {
        for own_hand in [0, 2] {
            let mut game = game();
            let mut untouched_graves = Vec::new();
            for (player, count) in [(A, own_hand), (B, 3), (C, 0)] {
                library(&mut game, player, 10);
                for index in 0..count {
                    vanilla(&mut game, player, Zone::Hand, &format!("Hand {index}"), false);
                }
                untouched_graves.push(vanilla(
                    &mut game, player, Zone::Graveyard, "Untouched grave", false,
                ));
            }
            let opponents_before: Vec<_> = [B, C]
                .into_iter()
                .map(|player| {
                    let player = game.player(player).unwrap();
                    (player.hand.clone(), player.library.clone())
                })
                .collect();
            let ability_index = definition
                .abilities
                .iter()
                .position(|ability| matches!(&ability.kind, AbilityKind::Activated(_)))
                .unwrap();
            let mut dm = Choices::default();
            let mana = game.player(A).unwrap().mana_pool.total();
            let spell = cast(&mut game, &definition, &mut dm);
            let identity = game.object(spell).unwrap().stable_id;
            assert_eq!(mana - game.player(A).unwrap().mana_pool.total(), 3);
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(A).unwrap().hand.len(), own_hand);

            resolve_one(&mut game, &mut dm);
            let warrior = game.find_object_by_stable_id(identity).unwrap();
            assert_eq!(game.object(warrior).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.stack.len(), 1, "the actual entry creates its trigger");
            assert!(game.stack[0].is_ability);
            assert!(shuffles(&game).is_empty(), "entry alone has not resolved the trigger");
            resolve_one(&mut game, &mut dm);
            assert!(game.stack.is_empty());
            assert_eq!(game.player(A).unwrap().hand.len(), own_hand);
            assert_eq!(game.player(A).unwrap().library.len(), 10);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(A), own_hand as u32);
            assert_eq!(shuffles(&game), vec![A]);
            for (player, (hand, library)) in [B, C].into_iter().zip(&opponents_before) {
                assert_eq!(&game.player(player).unwrap().hand, hand);
                assert_eq!(&game.player(player).unwrap().library, library);
                assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(player), 0);
            }

            let action = LegalAction::ActivateAbility { source: warrior, ability_index };
            game.turn.priority_player = Some(A);
            game.player_mut(A).unwrap().mana_pool.empty();
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
            assert_eq!(game.object(warrior).unwrap().zone, Zone::Battlefield);
            game.player_mut(A).unwrap().mana_pool.empty();
            game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red, 1);
            activate(&mut game, warrior, ability_index, &mut dm);
            let sacrificed = game.find_object_by_stable_id(identity).unwrap();
            assert_eq!(game.object(sacrificed).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.stack.len(), 1);
            assert!(game.stack[0].is_ability);
            assert_eq!(game.stack[0].mana_spent_on_activation.amount(ManaSymbol::Red), 1);
            assert_eq!(shuffles(&game), vec![A], "cost payment precedes the body");
            for (player, count) in [(A, own_hand), (B, 3), (C, 0)] {
                assert_eq!(game.player(player).unwrap().hand.len(), count);
            }

            resolve_one(&mut game, &mut dm);
            assert!(game.stack.is_empty());
            assert_eq!(shuffles(&game), vec![A, A, B, C]);
            for (player, count) in [(A, own_hand), (B, 3), (C, 0)] {
                assert_eq!(game.player(player).unwrap().hand.len(), count);
                assert_eq!(game.player(player).unwrap().library.len(), 10);
                assert_eq!(
                    game.turn_store.turn_history.cards_drawn_by_player(player),
                    (if player == A { count * 2 } else { count }) as u32
                );
            }
            for grave in untouched_graves {
                assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            }
            assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
            assert_eq!(game.object(sacrificed).unwrap().zone, Zone::Graveyard);
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| {
                matches!(action, LegalAction::ActivateAbility { source, .. } if *source == sacrificed)
            }));
        }
    }
}
#[test]
fn rite_keeps_target_player_distinct_from_both_sets_of_target_cards() {
    for definition in definitions("Rite of Renewal") {
        let mut game = game();
        library(&mut game, A, 5);
        library(&mut game, B, 5);
        let own = vanilla(&mut game, A, Zone::Graveyard, "Own permanent", true);
        let theirs = vanilla(&mut game, B, Zone::Graveyard, "Their card", false);
        let mut dm = Choices {
            targets: vec![
                Target::Object(own),
                Target::Player(B),
                Target::Object(theirs),
            ],
            ..Default::default()
        };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(game.player(B).unwrap().library.len(), 6);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 0);
        assert_eq!(shuffles(&game), vec![B]);
        assert!(game.exile.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name.as_ref() == "Rite of Renewal")
        }));
    }
}
#[test]
fn visions_optional_shuffle_uses_the_looked_at_library_owner() {
    for definition in definitions("Visions") {
        for accept in [false, true] {
            let mut game = game();
            library(&mut game, A, 8);
            library(&mut game, B, 8);
            let before = game.player(A).unwrap().library.clone();
            let looked_at = game.player(B).unwrap().library.iter().rev().take(5).copied().collect::<Vec<_>>();
            let mut dm = Choices {
                targets: vec![Target::Player(B)],
                accept,
                ..Default::default()
            };
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().library, before);
            assert_eq!(shuffles(&game), if accept { vec![B] } else { vec![] });
            assert_eq!(game.player(B).unwrap().library.len(), 8);
            assert_eq!(dm.views, vec![(A, B, Zone::Library, false, looked_at)]);
        }
    }
}

#[test]
fn molten_psyche_checks_metalcraft_and_the_whole_turns_draw_history_per_opponent() {
    for definition in definitions("Molten Psyche") {
        for artifacts in [2, 3] {
            let mut game = game();
            for player in [A, B, C] { library(&mut game, player, 12); }
            for _ in 0..2 { vanilla(&mut game, B, Zone::Hand, "Old hand", false); }
            let source = vanilla(&mut game, A, Zone::Battlefield, "Earlier draw source", true);
            let mut earlier = Choices::default();
            for (player, count) in [(B, 2), (C, 1)] {
                let mut ctx = ironsmith::effects::EffectContext::new(source, A, &mut earlier);
                ironsmith::effects::execute_effect(&mut game,
                    &ironsmith::effect::Effect::target_draws(count, ironsmith::target::PlayerFilter::Specific(player)),
                    &mut ctx).unwrap();
            }
            for _ in 0..artifacts {
                let artifact = ironsmith::CardBuilder::new(ironsmith::CardId::new(), "Artifact")
                    .card_types(vec![ironsmith::CardType::Artifact]).build();
                game.create_object_from_card(&artifact, A, Zone::Battlefield);
            }
            let mut dm = Choices::default();
            cast(&mut game, &definition, &mut dm);
            resolve(&mut game, &mut dm);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(B), 6);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(C), 2);
            assert_eq!(game.player(B).unwrap().hand.len(), 4);
            assert_eq!(game.player(C).unwrap().hand.len(), 1);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, if artifacts == 3 { 14 } else { 20 });
            assert_eq!(game.player(C).unwrap().life, if artifacts == 3 { 18 } else { 20 });
            assert_eq!(shuffles(&game), vec![A, B, C]);
        }
    }
}

#[test]
fn rite_accepts_zero_optional_card_targets_and_still_exiles_itself() {
    for definition in definitions("Rite of Renewal") {
        let mut game = game();
        library(&mut game, B, 3);
        let own = vanilla(&mut game, A, Zone::Graveyard, "Unchosen own permanent", true);
        let theirs = vanilla(&mut game, B, Zone::Graveyard, "Unchosen other card", false);
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        let spell = cast(&mut game, &definition, &mut dm);
        let identity = game.object(spell).unwrap().stable_id;
        resolve(&mut game, &mut dm);
        assert_eq!(game.object(own).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(theirs).unwrap().zone, Zone::Graveyard);
        let exiled = game.find_object_by_stable_id(identity).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
        assert_eq!(shuffles(&game), vec![B]);
        assert_eq!(game.player(B).unwrap().library.len(), 3);
    }
}

//! Full frozen bodies on independent compiler and serialized artifact routes.
//! Source-authored only; these scenarios have not been run in this campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext, ViewCardsContext};
use ironsmith::effects::{EffectContext, EffectExecutor, ExecutionError, RollDieEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn direct_definition(name: &str, text: &str) -> CardDefinition {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let definition = result.unwrap_or_else(|error| panic!("{name} independent direct: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    definition
}
fn artifact_definition(name: &str, text: &str) -> CardDefinition {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    // The artifact compiler's companion definition is not the direct route.
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    assert_eq!(artifact.diagnostics.error_count, 0);
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    let restored = materialize_artifact(&decoded).unwrap();
    assert_eq!(restored.card.name, decoded.card.name);
    assert_eq!(restored.canonical_text, decoded.payload.canonical_text);
    assert_eq!(restored.ability_labels, decoded.payload.ability_labels);
    restored
}
fn program_definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = direct_definition(name, text);
    let restored = artifact_definition(name, text);
    assert_eq!(direct.canonical_text, restored.canonical_text, "{name}");
    assert_eq!(direct.ability_labels, restored.ability_labels, "{name}");
    for definition in [&direct, &restored] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        let rendered = ironsmith_text::canonical_compiled_lines(definition).join("\n");
        assert!(!rendered.contains("tagged-object-reference"), "{name}: {rendered}");
        if text.to_ascii_lowercase().contains("roll") {
            assert!(rendered.to_ascii_lowercase().contains("roll"), "{name}: {rendered}");
        }
        if text.contains("and add") { assert!(rendered.contains("and add"), "{name}: {rendered}"); }
    }
    [direct, restored]
}
const ORIGINAL_TABLES: [(&str, &str, &str); 4] = [
    ("Diviner's Portent", "119585c7-ddfa-47ed-b2f8-488ebc156222", "{X}{U}{U}{U}"),
    ("Druid of the Emerald Grove", "acf54a85-0e9e-43fb-99d9-c223c02f13c4", "{3}{G}"),
    ("Song of Inspiration", "bdb80d7b-672c-4e2a-b93b-9b96721b93f2", "{3}{G}{G}"),
    ("Wyll's Reversal", "8d35cef8-a52d-45fb-8f5f-cccea26826d0", "{2}{R}"),
];
fn assert_original_metadata(definition: &CardDefinition, name: &str, mana: &str) {
    assert_eq!(definition.card.name, name);
    assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), mana);
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    let druid = name == "Druid of the Emerald Grove";
    assert_eq!(definition.card.card_types, vec![if druid { ironsmith::CardType::Creature } else { ironsmith::CardType::Instant }]);
    if druid {
        let pt = definition.card.power_toughness.as_ref().unwrap();
        assert_eq!((pt.power.base_value(), pt.toughness.base_value()), (2, 2));
        assert_eq!(definition.card.subtypes, vec![ironsmith::Subtype::Dwarf, ironsmith::Subtype::Druid]);
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 1);
        assert!(matches!(definition.abilities[0].kind, AbilityKind::Triggered(_)));
    } else {
        assert!(definition.card.power_toughness.is_none());
        assert!(definition.abilities.is_empty(), "a numeric row became a permanent ability: {name}");
        assert!(!definition.spell_effect.as_ref().unwrap().flattened_default_effects().is_empty());
    }
    assert!(!definition.canonical_text.contains("Station"));
    assert!(!definition.canonical_text.contains("tagged-object-reference"));
    assert!(definition.canonical_text.to_ascii_lowercase().contains("roll a d20"));
}
fn original_route_metadata(compile: fn(&str, &str) -> CardDefinition) {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/die_result_programs.json.fixture")).unwrap();
    for (name, id, mana) in ORIGINAL_TABLES {
        let row = rows.iter().find(|row| row["oracle_id"] == id).unwrap();
        assert_eq!(row["name"], name);
        let text = row["text"].as_str().unwrap();
        assert!(text.ends_with(row["oracle_text"].as_str().unwrap()));
        let definition = compile(name, text);
        assert_original_metadata(&definition, name, mana);
    }
}
#[test]
fn four_original_frozen_bodies_compile_independently_to_runtime_with_full_metadata() {
    original_route_metadata(direct_definition);
}
#[test]
fn four_original_frozen_bodies_compile_separately_through_validated_json_artifacts() {
    original_route_metadata(artifact_definition);
}

fn collect_runtime_nodes(effect: &ironsmith::Effect, nodes: &mut Vec<ironsmith::Effect>) {
    nodes.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect_runtime_nodes(child, nodes));
}

fn exact_die_instruction(mut effect: &ironsmith::Effect) -> bool {
    while let Some(inner) = effect.transparent_child_effect() { effect = inner; }
    effect.downcast_ref::<RollDieEffect>().is_some()
}

#[test]
fn original_full_body_tables_read_the_exact_die_instruction_and_portent_quantities_keep_paid_x() {
    for (name, _, _) in ORIGINAL_TABLES {
        for definition in definitions(name) {
            let program = if name == "Druid of the Emerald Grove" {
                let AbilityKind::Triggered(trigger) = &definition.abilities[0].kind else { panic!("Druid trigger missing"); };
                &trigger.effects
            } else {
                definition.spell_effect.as_ref().unwrap()
            };
            let mut nodes = Vec::new();
            for effect in program.all_effects() { collect_runtime_nodes(effect, &mut nodes); }
            let rolls = nodes.iter().filter(|effect| effect.downcast_ref::<RollDieEffect>().is_some()).count();
            assert_eq!(rolls, 1, "{name}: one physical die instruction");
            let gates = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::IfEffect>())
                .filter(|gate| matches!(gate.predicate, ironsmith::effect::EffectPredicate::Value(_)))
                .collect::<Vec<_>>();
            assert_eq!(gates.len(), if name == "Druid of the Emerald Grove" { 3 } else { 2 }, "{name}");
            let die_id = gates[0].condition;
            assert!(gates.iter().all(|gate| gate.condition == die_id), "{name}: sibling rows must share one result");
            let producers = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::WithIdEffect>())
                .filter(|producer| producer.id == die_id).collect::<Vec<_>>();
            assert_eq!(producers.len(), 1, "{name}: one exact producer for all rows");
            assert!(exact_die_instruction(&producers[0].effect), "{name}: a draw/search/aggregate cannot own the die result");
            if name == "Diviner's Portent" {
                let draws = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>()).collect::<Vec<_>>();
                let scries = nodes.iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::ScryEffect>()).collect::<Vec<_>>();
                assert_eq!(draws.len(), 2);
                assert_eq!(scries.len(), 1);
                assert!(draws.iter().all(|draw| matches!(draw.count.unhinted(), ironsmith::effect::Value::X)));
                assert!(matches!(scries[0].count.unhinted(), ironsmith::effect::Value::X));
            }
        }
    }
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/die_result_programs.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    program_definitions(name, row["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 30);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for player in [A, B] {
        for symbol in [ironsmith::ManaSymbol::White, ironsmith::ManaSymbol::Blue, ironsmith::ManaSymbol::Black, ironsmith::ManaSymbol::Red, ironsmith::ManaSymbol::Green, ironsmith::ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
    }
    game
}
fn assert_die_receipt(game: &GameState, natural: u32, modified: u32) {
    let receipts = game.turn_store.turn_history.event_records.iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter_map(|record| record.event.downcast::<ironsmith::events::other::DieRolledEvent>())
        .map(|event| (event.player, event.natural_result, event.result, event.sides, event.ordinal_this_turn))
        .collect::<Vec<_>>();
    assert_eq!(receipts, vec![(A, natural, modified, 20, Some(1))]);
    assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
    assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [modified]);
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    game.create_object_from_definition(&compile_to_runtime_definition(name, text, false).unwrap(), owner, zone)
}
#[derive(Default)]
struct Choices {
    x: u32,
    select_maximum: Option<usize>,
    preferred: Option<ObjectId>,
    accept: bool,
    options: std::collections::VecDeque<usize>,
    targets: std::collections::VecDeque<Vec<Target>>,
    views: Vec<(PlayerId, bool, Vec<ObjectId>)>,
    viewed_hand_sizes: Vec<usize>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.targets.pop_front().unwrap_or_else(|| SelectFirstDecisionMaker.decide_targets(game, context))
    }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let limit = self.select_maximum.unwrap_or(usize::MAX).min(context.max.unwrap_or(context.candidates.len()));
        let mut candidates = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect::<Vec<_>>();
        candidates.sort_by_key(|id| Some(*id) != self.preferred);
        candidates.into_iter().take(limit).collect()
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { self.x } else { SelectFirstDecisionMaker.decide_number(game, context) }
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        self.options.pop_front().map(|choice| vec![choice]).unwrap_or_else(|| SelectFirstDecisionMaker.decide_options(game, context))
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
    }
    fn view_cards(&mut self, game: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        self.views.push((viewer, context.public, cards.to_vec()));
        self.viewed_hand_sizes.push(game.player(viewer).unwrap().hand.len());
    }
}
fn action(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    action_for(game, A, action, dm);
}
fn action_for(game: &mut GameState, player: PlayerId, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn cast(game: &mut GameState, spell: ObjectId, dm: &mut Choices) {
    action(game, LegalAction::CastSpell { spell_id: spell, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, dm);
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..32 {
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    }
    panic!("stack did not settle");
}
#[test]
fn investigator_pays_full_labeled_cost_and_each_table_branch_owns_its_complete_partition() {
    for definition in definitions("Arcane Investigator") {
        assert_eq!(definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Activated(_))).count(), 1);
        let ability_index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        for result in [1, 9, 10, 20] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let bottom = object(&mut game, A, Zone::Library, "Bottom", "Type: Land");
            let third = object(&mut game, A, Zone::Library, "Third", "Type: Land");
            let middle = object(&mut game, A, Zone::Library, "Middle", "Type: Land");
            let top = object(&mut game, A, Zone::Library, "Top", "Type: Land");
            let before = game.player(A).unwrap().mana_pool.total();
            let mut dm = Choices { preferred: Some(middle), ..Default::default() };
            game.force_next_die_roll(result);
            action(&mut game, LegalAction::ActivateAbility { source, ability_index }, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 6);
            assert!(game.player(A).unwrap().hand.is_empty());
            settle(&mut game, &mut dm);
            assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            let card = game.object(game.player(A).unwrap().hand[0]).unwrap();
            assert_eq!(card.name, if result < 10 { "Top" } else { "Middle" });
            assert_eq!(game.player(A).unwrap().library.len(), 3);
            assert!(game.object(bottom).is_some());
            if result >= 10 {
                assert_eq!(game.player(A).unwrap().library.last(), Some(&bottom));
                assert!(dm.views.iter().any(|(viewer, public, cards)| *viewer == A && !public && cards.contains(&top) && cards.contains(&middle) && cards.contains(&third)));
                assert!(!dm.views.iter().any(|(viewer, public, _)| *viewer == B || *public));
            }
        }
    }
}
#[test]
fn herald_keeps_life_gain_and_treasure_tail_inside_the_correct_paid_row() {
    for definition in definitions("Herald of Hadar") {
        let ability_index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        for result in [1, 9, 10, 19, 20] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let before = game.player(A).unwrap().mana_pool.total();
            let mut dm = Choices::default();
            game.force_next_die_roll(result);
            action(&mut game, LegalAction::ActivateAbility { source, ability_index }, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 6);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, if result >= 10 { 32 } else { 30 });
            assert_eq!(game.player(B).unwrap().life, 28);
            assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().name == "Treasure").count(), if result == 20 { 2 } else { 0 });
            assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
        }
    }
}
#[test]
fn druid_preserves_revealed_search_selection_across_roll_and_all_three_rows() {
    for definition in definitions("Druid of the Emerald Grove") {
        for result in [1, 9, 10, 19, 20] {
            for selected in [0, 1, 2] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Hand);
                let original_lands = (0..3).map(|index| {
                    let name = format!("Forest {index}");
                    (object(&mut game, A, Zone::Library, &name, "Type: Basic Land — Forest"), name)
                }).collect::<Vec<_>>();
                let excluded = object(&mut game, A, Zone::Library, "Nonbasic", "Type: Land");
                let before = game.player(A).unwrap().mana_pool.total();
                let mut dm = Choices { select_maximum: Some(selected), ..Default::default() };
                game.force_next_die_roll(result);
                cast(&mut game, source, &mut dm);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 4);
                settle(&mut game, &mut dm);
                let battlefield = if result <= 9 { 0 } else if result <= 19 { selected.min(1) } else { selected };
                assert_eq!(game.player(A).unwrap().hand.len(), selected - battlefield);
                assert_eq!(game.player(A).unwrap().library.len(), 4 - selected);
                assert_eq!(game.object(excluded).unwrap().zone, Zone::Library);
                let lands = game.battlefield.iter().filter(|id| game.object(**id).unwrap().name.starts_with("Forest ")).copied().collect::<Vec<_>>();
                assert_eq!(lands.len(), battlefield);
                assert!(lands.into_iter().all(|id| game.is_tapped(id)));
                assert_die_receipt(&game, result, result);
                let revealed = dm.views.iter().filter(|(_, public, _)| *public)
                    .flat_map(|(_, _, cards)| cards.iter().copied()).collect::<std::collections::HashSet<_>>();
                assert_eq!(revealed.len(), selected, "only the search selection is revealed");
                for (original, name) in &original_lands {
                    if revealed.contains(original) {
                        let destinations = game.player(A).unwrap().hand.iter().chain(game.battlefield.iter())
                            .filter(|id| game.object(**id).unwrap().name == *name).count();
                        assert_eq!(destinations, 1, "every selected land moves exactly once: {name}");
                    } else {
                        assert_eq!(game.object(*original).unwrap().zone, Zone::Library);
                    }
                }
                let shuffles = game.turn_store.turn_history.event_records.iter()
                    .chain(game.turn_store.turn_history.staged_event_records.iter())
                    .filter_map(|record| record.event.downcast::<ironsmith::events::ShuffleLibraryEvent>())
                    .map(|event| event.player).collect::<Vec<_>>();
                assert_eq!(shuffles, vec![A], "each row completes the deferred search shuffle once");
                if selected > 0 { assert!(dm.views.iter().any(|(_, public, cards)| *public && cards.len() == selected)); }
            }
        }
    }
}
#[test]
fn portent_counts_remaining_hand_after_cast_and_keeps_announced_x_for_both_rows() {
    for definition in definitions("Diviner's Portent") {
        for (natural, hand_size, x) in [(14, 0, 2), (14, 1, 3), (10, 5, 1), (20, 5, 2), (20, 0, 0), (1, 0, 0), (1, 0, 5), (20, 0, 5)] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            for index in 0..hand_size { object(&mut game, A, Zone::Hand, &format!("Held {index}"), "Type: Land"); }
            for index in 0..8 { object(&mut game, A, Zone::Library, &format!("Draw {index}"), "Type: Land"); }
            let before = game.player(A).unwrap().mana_pool.total();
            let mut dm = Choices { x, ..Default::default() };
            game.force_next_die_roll(natural);
            cast(&mut game, spell, &mut dm);
            assert_eq!(game.stack.last().unwrap().x_value, Some(x));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - x - 3);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), hand_size + x as usize);
            assert_die_receipt(&game, natural, natural + hand_size as u32);
            let scry_views = dm.views.iter().filter(|(_, public, cards)| !public && cards.len() == x as usize).count();
            if natural + hand_size as u32 >= 15 && x > 0 {
                assert!(scry_views > 0);
                assert!(dm.viewed_hand_sizes.iter().all(|size| *size == hand_size),
                    "the full-body high row scries before drawing its paid X cards");
            }
            else { assert_eq!(scry_views, 0); }
        }
    }
}
#[test]
fn malformed_die_suffixes_and_unbound_roll_quantities_fail_without_partial_artifacts() {
    for text in [
        "Roll a d20 banana.",
        "Roll a six-sided die banana.",
        "Roll a d20 and add the number of cards in your hand banana.",
        "Roll a d20 and subtract.",
        "Roll a d20 and add three {R}.",
        "Roll a d20 and add the number of cards {R} in your hand.",
        "Roll a d20 and add the number of cards in your hand:",
        "Roll a d20 and add three ???.",
        "Roll a d20 and add the toughness of a purple turn.",
        "Roll a d20 and add the result.",
    ] {
        let text = format!("Type: Sorcery\n{text}");
        assert!(compile_to_artifact("Incomplete die instruction", &text, false).is_err(), "{text}");
        assert!(compile_to_runtime_definition("Incomplete die instruction", &text, false).is_err(), "{text}");
    }
}
#[test]
fn numeric_rows_without_a_local_die_and_incomplete_headers_fail_on_both_public_routes() {
    for text in [
        "15+ | Draw a card.",
        "9 or less | Draw a card.",
        "15+ | Flying",
        "Draw a card.\n15+ | Draw two cards.",
        "Draw a card.\n9 or less | Draw two cards.",
        "{0}: Draw a card.\n15+ | Draw two cards.",
        "When this artifact enters, draw a card.\n15+ | Draw two cards instead.",
        "Roll a d20. Draw a card.\n15+ | You gain 5 life.",
        "Roll a d20, then draw a card.\n15+ | You gain 5 life.",
        "{0}: Roll a d20. Draw a card.\n15+ | You gain 5 life.",
        "Fortune — {0}: Roll a d20. Draw a card.\n15+ | You gain 5 life.",
        "When this artifact enters, roll a d20. Draw a card.\n15+ | You gain 5 life.",
        "You may roll a d20.\n15+ | You gain 5 life.",
        "If you control a creature, roll a d20.\n15+ | You gain 5 life.",
        "Target creature gains \"{T}: Roll a d20.\" until end of turn.\n15+ | You gain 5 life.",
        "Roll a d20.\n15 + banana | Draw a card.",
        "Roll a d20.\n9 or less banana | Draw a card.",
        "Roll a d20.\n1—14 banana | Draw a card.",
        "Roll a d20.\n14—1 | Draw a card.",
        "Roll a d20.\n15+ |",
        "Roll a d20.\n9 or less |",
        "Roll a d20.\n15+ | Draw a card. Purple the moon.",
        "Station\n9 or less | Flying",
        "Station\n15+ | Draw a card.",
        "Station\n3+ | Flying\n{T}: Roll a d20.\n9+ | You gain 5 life.",
    ] {
        for card_type in ["Sorcery", "Artifact"] {
            let text = format!("Mana cost: {{0}}\nType: {card_type}\n{text}");
            assert!(compile_to_runtime_definition("Unowned or incomplete numeric row", &text, false).is_err(), "direct: {text}");
            assert!(compile_to_artifact("Unowned or incomplete numeric row", &text, false).is_err(), "artifact: {text}");
        }
    }
}

#[test]
fn activated_paid_x_is_local_to_its_cost_and_does_not_inherit_the_permanents_printed_x() {
    for (printed, activation, announced, natural, expected_draw, expected_payment) in [
        ("{0}", "{X}{U}", 3, 7, 3, 4),
        ("{0}", "{X}{U}", 0, 20, 0, 1),
        ("{X}", "{1}", 0, 7, 7, 1),
    ] {
        let text = format!("Mana cost: {printed}\nType: Artifact\n{activation}: Roll a d20.\n1—20 | Draw X cards.");
        for definition in program_definitions("Activation X scope", &text) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for index in 0..24 { object(&mut game, A, Zone::Library, &format!("Library {index}"), "Type: Land"); }
            let before = game.player(A).unwrap().mana_pool.total();
            let mut dm = Choices { x: announced, ..Default::default() };
            game.force_next_die_roll(natural);
            action(&mut game, LegalAction::ActivateAbility { source, ability_index: 0 }, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - expected_payment);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), expected_draw);
            assert_die_receipt(&game, natural, natural);
        }
    }
}

#[test]
fn earlier_draws_do_not_own_a_later_table_and_rolls_inside_one_row_do_not_rebind_siblings() {
    for definition in program_definitions("Exact terminal die", "Mana cost: {0}\nType: Sorcery\nDraw a card, then roll a d20.\n1—14 | You gain 1 life.\n15+ | You gain 5 life.") {
        for (natural, life) in [(1, 31), (20, 35)] {
            let mut game = game();
            object(&mut game, A, Zone::Library, "Only draw", "Type: Land");
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices::default();
            game.force_next_die_roll(natural);
            cast(&mut game, spell, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 1);
            assert_eq!(game.player(A).unwrap().life, life);
            assert_die_receipt(&game, natural, natural);
        }
    }
    for definition in program_definitions("Latest direct die", "Mana cost: {0}\nType: Sorcery\nRoll a d20. Roll a d20.\n1—14 | You gain 1 life.\n15+ | You gain 5 life.") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices::default();
        game.force_next_die_roll(20);
        game.force_next_die_roll(1);
        cast(&mut game, spell, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 31);
        assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [20, 1]);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 2);
    }
    for definition in program_definitions("Row-local die", "Mana cost: {0}\nType: Sorcery\nRoll a d20.\n1—14 | Roll a d20.\n15+ | You gain 5 life.") {
        let mut game = game();
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices::default();
        game.force_next_die_roll(1);
        game.force_next_die_roll(20);
        cast(&mut game, spell, &mut dm);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 30);
        assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [1, 20]);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 2);
    }
}
#[test]
fn low_open_ended_row_includes_zero_and_high_row_is_not_capped_at_die_sides() {
    for (operation, natural, modified, life) in [("subtract twenty", 20, 0, 31), ("add twenty", 20, 40, 33)] {
        let text = format!("Mana cost: {{0}}\nType: Sorcery\nRoll a d20 and {operation}.\n9 or less | You gain 1 life.\n10—19 | You gain 2 life.\n20+ | You gain 3 life.");
        for definition in program_definitions("Open-ended result bounds", &text) {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choices::default();
            game.force_next_die_roll(natural);
            cast(&mut game, spell, &mut dm);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, life);
            assert_die_receipt(&game, natural, modified);
        }
    }
}
#[test]
fn eternity_elevator_retains_real_station_ownership_and_charge_gated_mana_in_both_routes() {
    // The complete body and metadata are retained from the existing native
    // The Eternity Elevator regression, independently on both public routes.
    let text = "Mana cost: {5}\nType: Legendary Artifact — Spacecraft\n{T}: Add {C}{C}{C}.\nStation (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery.)\n20+ | {T}: Add X mana of any one color, where X is the number of charge counters on The Eternity Elevator.";
    for definition in program_definitions("The Eternity Elevator", text) {
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 3);
        assert!(definition.canonical_text.contains("Station"));
        assert!(definition.canonical_text.contains("20+ |"));
        let threshold = definition.abilities.iter().position(|ability| matches!(&ability.kind,
            AbilityKind::Activated(activated) if activated.effects.flattened_default_effects().iter()
                .any(|effect| effect.downcast_ref::<ironsmith::effects::AddManaOfAnyOneColorEffect>().is_some())
        )).unwrap();
        for count in [0, 19, 20, 23] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            if count > 0 { game.add_counters(source, ironsmith::CounterType::Charge, count).unwrap(); }
            let legal = ironsmith::decision::compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
                LegalAction::ActivateManaAbility { source: id, ability_index } if *id == source && *ability_index == threshold
            ));
            assert_eq!(legal, count >= 20);
            if legal {
                let before = game.player(A).unwrap().mana_pool.total();
                action(&mut game, LegalAction::ActivateManaAbility { source, ability_index: threshold }, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().mana_pool.total(), before + count);
                assert!(game.is_tapped(source));
            }
            assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
        }
    }
}

#[test]
fn station_thresholds_keep_precedence_over_an_intervening_die_activation() {
    let text = "Mana cost: {0}\nType: Artifact\nStation\n3+ | Flying\n{T}: Roll a d20.\n9+ | Vigilance";
    for definition in program_definitions("Station with a roll", text) {
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 4);
        assert!(definition.canonical_text.contains("3+ |"));
        assert!(definition.canonical_text.contains("9+ |"));
        let roll_index = definition.abilities.iter().position(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else { return false; };
            let mut nodes = Vec::new();
            for effect in activated.effects.all_effects() { collect_runtime_nodes(effect, &mut nodes); }
            nodes.iter().any(|effect| effect.downcast_ref::<RollDieEffect>().is_some())
        }).expect("standalone die activation remains under the 3+ Station striation");
        for count in [0, 2, 3, 8, 9, 12] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            if count > 0 { game.add_counters(source, ironsmith::CounterType::Charge, count).unwrap(); }
            assert_eq!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying), count >= 3);
            assert_eq!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Vigilance), count >= 9);
            let available = ironsmith::decision::compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action,
                LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == roll_index
            ));
            assert_eq!(available, count >= 3);
            if available {
                let mut dm = Choices::default();
                game.force_next_die_roll(20);
                action(&mut game, LegalAction::ActivateAbility { source, ability_index: roll_index }, &mut dm);
                settle(&mut game, &mut dm);
                assert_die_receipt(&game, 20, 20);
                assert_eq!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Vigilance), count >= 9,
                    "the die result does not turn a Station striation into a result row");
            }
        }
    }
}
#[test]
fn arithmetic_uses_one_random_attempt_and_keeps_natural_and_modified_event_receipts_distinct() {
    use ironsmith::effect::{ExecutionFact, Value};
    use ironsmith_core::effect::DieResultModifier;
    let mut original = game();
    original.set_random_seed(711);
    let source = original.new_object_id();
    let mut modified = original.clone();
    let baseline = RollDieEffect::new(PlayerFilter::You, 20).execute(&mut original, &mut EffectContext::new_default(source, A)).unwrap();
    let mut effect = RollDieEffect::new(PlayerFilter::You, 20);
    effect.result_modifier = Some(DieResultModifier::Add(Value::Fixed(7)));
    let result = effect.execute(&mut modified, &mut EffectContext::new_default(source, A)).unwrap();
    let event = result.events.iter().find_map(|event| event.downcast::<ironsmith::events::other::DieRolledEvent>()).unwrap();
    assert_eq!(i64::from(event.natural_result), baseline.as_count().unwrap());
    assert_eq!(event.result, event.natural_result + 7);
    assert_eq!(result.as_count(), Some(i64::from(event.result)));
    assert!(result.execution_facts.contains(&ExecutionFact::ChosenNumber(event.result)));
    assert_eq!(modified.irreversible_random_count(), original.irreversible_random_count());
    assert_eq!(modified.turn_store.turn_history.completed_die_roll_count(A), 1);
    assert_eq!(modified.turn_store.turn_history.die_rolls_this_turn[&A], [event.result]);
}
#[test]
fn arithmetic_does_not_narrow_wide_results_or_confuse_negative_addition_and_subtraction() {
    use ironsmith::effect::Value;
    use ironsmith_core::effect::DieResultModifier::{Add, Subtract};
    for (modifier, expected) in [
        (Add(Value::Fixed(i32::MAX)), i32::MAX as u32 + 20),
        (Add(Value::Fixed(-5)), 15),
        (Subtract(Value::Fixed(5)), 15),
        (Subtract(Value::Fixed(21)), 0),
    ] {
        let mut game = game();
        let source = game.new_object_id();
        game.force_next_die_roll(20);
        let mut effect = RollDieEffect::new(PlayerFilter::You, 20);
        effect.result_modifier = Some(modifier);
        let outcome = effect.execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        assert_eq!(outcome.as_count(), Some(i64::from(expected)));
        assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [expected]);
    }
}
#[test]
fn arithmetic_resource_failure_rolls_back_the_whole_program_and_prior_instruction_receipts() {
    use ironsmith::effect::{Effect, EffectId, EffectOutcome, Value};
    use ironsmith::effects::{SequenceEffect, execute_effect};
    let mut game = game();
    let source = game.new_object_id();
    let mut roll = RollDieEffect::new(PlayerFilter::You, 20);
    roll.result_modifier = Some(ironsmith_core::effect::DieResultModifier::Add(Value::Scaled(Box::new(Value::Fixed(i32::MAX)), 3)));
    let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(4), Effect::with_id(7, Effect::new(roll))]));
    game.force_next_die_roll(20);
    let before = game.irreversible_random_count();
    let mut context = EffectContext::new_default(source, A);
    context.effect_outcomes.insert(EffectId(9), EffectOutcome::count(11));
    let error = execute_effect(&mut game, &sequence, &mut context).unwrap_err();
    assert!(matches!(error, ExecutionError::ResourceLimitExceeded { resource: "modified die result", .. }));
    assert_eq!(game.player(A).unwrap().life, 30);
    assert_eq!(game.irreversible_random_count(), before);
    assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
    assert_eq!(context.effect_outcomes.len(), 1);
    assert_eq!(context.effect_outcomes[&EffectId(9)].as_count(), Some(11));
    assert_eq!(game.take_forced_die_roll(), Some(20));
    assert!(game.take_pending_trigger_events().is_empty());
}
struct PauseModifierOrder { pause: bool, pending: bool }
impl DecisionMaker for PauseModifierOrder {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_options(&mut self, _: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        self.pending = self.pause;
        vec![context.options.last().unwrap().index]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { false }
}
#[test]
fn authored_and_external_modifier_order_suspends_and_replays_the_whole_program_atomically() {
    use ironsmith::effect::{Effect, Value};
    use ironsmith::effects::{SequenceEffect, execute_effect};
    let mut game = game();
    let source = object(&mut game, A, Zone::Battlefield, "External modifier", "Type: Artifact\nAfter you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.");
    let mut roll = RollDieEffect::new(PlayerFilter::You, 20);
    roll.result_modifier = Some(ironsmith_core::effect::DieResultModifier::Add(Value::Fixed(3)));
    let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(4), Effect::new(roll)]));
    game.force_next_die_roll(12);
    let before = game.irreversible_random_count();
    let mut dm = PauseModifierOrder { pause: true, pending: false };
    let pending = execute_effect(&mut game, &sequence, &mut EffectContext::new(source, A, &mut dm)).unwrap();
    assert!(pending.events.is_empty());
    assert_eq!(game.player(A).unwrap().life, 30);
    assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
    assert_eq!(game.irreversible_random_count(), before);
    dm.pause = false;
    dm.pending = false;
    let result = execute_effect(&mut game, &sequence, &mut EffectContext::new(source, A, &mut dm)).unwrap();
    assert_eq!(game.player(A).unwrap().life, 34);
    assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [15]);
    assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
    assert_eq!(result.events.iter().filter(|event| event.downcast::<ironsmith::events::other::DieRolledEvent>().is_some()).count(), 1);
    assert_eq!(game.irreversible_random_count(), before);
}
#[test]
fn modified_local_result_in_triggered_program_does_not_read_the_triggering_die_or_later_life_gain() {
    for definition in program_definitions("Local arithmetic trigger", "Type: Artifact\nWhenever you roll a 1, roll a d20 and add the number of cards in your hand. You gain 2 life. If the roll was 15 or higher, draw a card.") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        object(&mut game, A, Zone::Hand, "Held", "Type: Land");
        object(&mut game, A, Zone::Library, "Reward", "Type: Land");
        game.force_next_die_roll(1);
        let mut dm = Choices::default();
        let triggering = RollDieEffect::new(PlayerFilter::You, 6).execute(&mut game, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        for event in triggering.events { game.queue_trigger_event(Default::default(), event); }
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        game.force_next_die_roll(14);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(A).unwrap().life, 32);
        assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [1, 15]);
    }
}
#[test]
fn bag_uses_the_local_roll_to_stop_public_reveal_and_randomizes_only_the_remainder() {
    for definition in definitions("Bag of Tricks") {
        let ability_index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        for (result, hit) in [(1, true), (8, true), (4, false)] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let untouched = object(&mut game, A, Zone::Library, "Unexposed bottom", "Type: Land");
            let matching = object(&mut game, A, Zone::Library, "Matching creature", &format!("Mana cost: {{{}}}\nType: Creature — Beast\nPower/Toughness: 2/2", if hit { result } else { result + 1 }));
            let wrong_type = object(&mut game, A, Zone::Library, "Matching value artifact", &format!("Mana cost: {{{result}}}\nType: Artifact"));
            let wrong_value = object(&mut game, A, Zone::Library, "Wrong value creature", "Mana cost: {9}\nType: Creature — Beast\nPower/Toughness: 2/2");
            let before = game.player(A).unwrap().mana_pool.total();
            let mut dm = Choices::default();
            game.force_next_die_roll(result);
            action(&mut game, LegalAction::ActivateAbility { source, ability_index }, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 5);
            assert!(game.is_tapped(source));
            settle(&mut game, &mut dm);
            assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
            assert_eq!(game.player(A).unwrap().library.len(), if hit { 3 } else { 4 });
            assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().name == "Matching creature").count(), usize::from(hit));
            assert!(game.player(A).unwrap().hand.is_empty());
            assert!(game.object(wrong_type).is_some());
            assert!(game.object(wrong_value).is_some());
            let revealed = dm.views.iter().filter(|(_, public, _)| *public).flat_map(|(_, _, cards)| cards.iter()).copied().collect::<std::collections::HashSet<_>>();
            assert!(revealed.contains(&wrong_type) && revealed.contains(&wrong_value) && revealed.contains(&matching));
            if hit {
                assert!(!revealed.contains(&untouched));
                assert_eq!(game.player(A).unwrap().library.last(), Some(&untouched));
            } else { assert!(revealed.contains(&untouched)); }
        }
    }
}
#[test]
fn reversal_uses_saved_stack_target_and_power_then_independently_retargets_original_and_copy() {
    for definition in definitions("Wyll's Reversal") {
        for (natural, power, copy) in [(14, None, false), (14, Some(1), true), (20, Some(-6), false), (20, Some(4), true)] {
            let mut game = game();
            let original = object(&mut game, B, Zone::Hand, "Original bolt", "Mana cost: {R}\nType: Instant\nThis spell deals 3 damage to target player.");
            let reversal = game.create_object_from_definition(&definition, A, Zone::Hand);
            if let Some(power) = power { object(&mut game, A, Zone::Battlefield, "Arithmetic creature", &format!("Type: Creature — Beast\nPower/Toughness: {power}/3")); }
            if power == Some(4) { object(&mut game, A, Zone::Battlefield, "Smaller controlled creature", "Type: Creature — Beast\nPower/Toughness: 3/3"); }
            // A larger opposing creature is outside the arithmetic operand.
            object(&mut game, B, Zone::Battlefield, "Opposing creature", "Type: Creature — Beast\nPower/Toughness: 20/20");
            let mut dm = Choices { accept: true, targets: vec![vec![Target::Player(A)]].into(), ..Default::default() };
            action_for(&mut game, B, LegalAction::CastSpell { spell_id: original, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
            let original_stack = game.stack.last().unwrap().target_id();
            dm.targets.push_back(vec![Target::Object(original_stack)]);
            let mana = game.player(A).unwrap().mana_pool.total();
            cast(&mut game, reversal, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 3);
            assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(original_stack)]);
            game.force_next_die_roll(natural);
            dm.targets.push_back(vec![Target::Player(B)]);
            if copy { dm.targets.push_back(vec![Target::Player(A)]); }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_die_receipt(&game, natural, (natural as i32 + power.unwrap_or(0)).max(0) as u32);
            assert_eq!(game.stack.len(), if copy { 2 } else { 1 });
            let original_entry = game.stack.iter().find(|entry| entry.target_id() == original_stack).unwrap();
            assert_eq!(original_entry.controller, B);
            assert_eq!(original_entry.targets, vec![Target::Player(B)]);
            if copy {
                let copied = game.stack.last().unwrap();
                assert_ne!(copied.target_id(), original_stack);
                assert_eq!(copied.controller, A);
                assert_eq!(copied.targets, vec![Target::Player(A)]);
                assert_eq!(game.object(copied.object_id).unwrap().name, "Original bolt");
            }
            settle(&mut game, &mut dm);
            assert_eq!(game.player(B).unwrap().life, 27);
            assert_eq!(game.player(A).unwrap().life, if copy { 27 } else { 30 });
        }
    }
}
#[test]
fn reversal_can_copy_the_exact_activated_ability_after_its_source_leaves_and_declining_retarget_does_not_skip_copy() {
    for definition in definitions("Wyll's Reversal") {
        let mut game = game();
        let source = object(&mut game, B, Zone::Battlefield, "Departed ability source", "Type: Artifact\n{0}: This artifact deals 3 damage to target player.");
        let reversal = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices { targets: vec![vec![Target::Player(A)]].into(), ..Default::default() };
        action_for(&mut game, B, LegalAction::ActivateAbility { source, ability_index: 0 }, &mut dm);
        let original_stack = game.stack.last().unwrap().target_id();
        dm.targets.push_back(vec![Target::Object(original_stack)]);
        cast(&mut game, reversal, &mut dm);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        game.force_next_die_roll(20);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_die_receipt(&game, 20, 20);
        assert_eq!(game.stack.len(), 2);
        let original = game.stack.iter().find(|entry| entry.target_id() == original_stack).unwrap();
        assert!(original.is_ability);
        assert_eq!(original.controller, B);
        let copy = game.stack.last().unwrap();
        assert!(copy.is_ability);
        assert_ne!(copy.target_id(), original_stack);
        assert_eq!(copy.controller, A);
        assert_eq!(copy.targets, vec![Target::Player(A)]);
        settle(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 24);
        assert_eq!(game.player(B).unwrap().life, 30);
    }
}
#[test]
fn reversal_excludes_targetless_stack_entries_and_fizzles_when_its_only_target_leaves() {
    for definition in definitions("Wyll's Reversal") {
        let mut game = game();
        let targetless = object(&mut game, B, Zone::Hand, "Targetless spell", "Mana cost: {0}\nType: Instant\nYou gain 1 life.");
        let reversal = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices::default();
        action_for(&mut game, B, LegalAction::CastSpell { spell_id: targetless, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
        assert!(!ironsmith::decision::compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == reversal)));
        settle(&mut game, &mut dm);
        let bolt = object(&mut game, B, Zone::Hand, "Vanishing spell", "Mana cost: {0}\nType: Instant\nThis spell deals 3 damage to target player.");
        dm.targets.push_back(vec![Target::Player(A)]);
        action_for(&mut game, B, LegalAction::CastSpell { spell_id: bolt, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal }, &mut dm);
        let target = game.stack.last().unwrap().target_id();
        dm.targets.push_back(vec![Target::Object(target)]);
        cast(&mut game, reversal, &mut dm);
        game.move_object_by_effect(target, Zone::Exile).unwrap();
        game.force_next_die_roll(20);
        settle(&mut game, &mut dm);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
        assert_eq!(game.take_forced_die_roll(), Some(20));
        assert_eq!(game.player(A).unwrap().life, 30);
    }
}
struct ConsultOpenings {
    pause_at: Option<usize>,
    pending: bool,
    opened: Vec<ObjectId>,
    viewed: Vec<ObjectId>,
}
impl DecisionMaker for ConsultOpenings {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.player, A);
        assert_eq!((context.min, context.max), (1, Some(1)));
        assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
        assert_eq!(context.candidates.len(), 1, "only the next examined card may be opened");
        let id = context.candidates[0].id;
        self.opened.push(id);
        self.pending = self.pause_at == Some(self.opened.len());
        vec![id]
    }
    fn view_cards(&mut self, game: &GameState, _: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert!(context.public);
        assert_eq!(cards.len(), 1);
        assert!(!game.is_hidden_card_placeholder(cards[0]), "no placeholder is evidence of a nonmatch");
        self.viewed.extend_from_slice(cards);
    }
}
#[test]
fn bag_publicly_opens_each_examined_identity_before_matching_and_never_opens_the_suffix() {
    for definition in definitions("Bag of Tricks") {
        for wrong_type_first in [false, true] {
            for identity_already_known in [false, true] {
                let mut game = game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let untouched = game.create_hidden_card_placeholder(A, Zone::Library, 31, "unexamined-library-suffix".into());
                let hit = game.create_hidden_card_placeholder(A, Zone::Library, 32, "first-matching-creature".into());
                let hit_stable = game.object(hit).unwrap().stable_id;
                let creature = compile_to_runtime_definition("Opened creature", "Mana cost: {3}\nType: Creature — Beast\nPower/Toughness: 2/2", false).unwrap();
                let wrong = wrong_type_first.then(|| game.create_hidden_card_placeholder(A, Zone::Library, 33, "same-value-wrong-type".into()));
                let artifact = compile_to_runtime_definition("Opened artifact", "Mana cost: {3}\nType: Artifact", false).unwrap();
                if identity_already_known {
                    game.reveal_hidden_card_with_definition(hit, &creature).unwrap();
                    if let Some(wrong) = wrong { game.reveal_hidden_card_with_definition(wrong, &artifact).unwrap(); }
                }
                let index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
                action(&mut game, LegalAction::ActivateAbility { source, ability_index: index }, &mut Choices::default());
                let paid = game.player(A).unwrap().mana_pool.total();
                let library = game.player(A).unwrap().library.clone();
                let before = game.irreversible_random_count();
                game.force_next_die_roll(3);
                let mut dm = ConsultOpenings { pause_at: Some(1), pending: false, opened: Vec::new(), viewed: Vec::new() };
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert!(dm.pending);
                assert_eq!(dm.opened, vec![wrong.unwrap_or(hit)]);
                assert!(dm.viewed.is_empty());
                assert_eq!(game.player(A).unwrap().library, library);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), paid);
                assert!(game.is_tapped(source));
                assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
                assert_eq!(game.irreversible_random_count(), before);
                assert!(game.publicly_revealed_hidden_cards().is_empty());
                game.reveal_hidden_card_with_definition(hit, &creature).unwrap();
                if let Some(wrong) = wrong { game.reveal_hidden_card_with_definition(wrong, &artifact).unwrap(); }
                dm.pause_at = None;
                dm.pending = false;
                dm.opened.clear();
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert!(game.stack.is_empty());
                assert_eq!(dm.opened, wrong.into_iter().chain(std::iter::once(hit)).collect::<Vec<_>>());
                assert!(!dm.opened.contains(&untouched) && !dm.viewed.contains(&untouched));
                assert!(game.is_hidden_card_placeholder(untouched));
                assert!(!game.is_publicly_revealed_hidden_card(untouched));
                assert_eq!(game.object(game.find_object_by_stable_id(hit_stable).unwrap()).unwrap().zone, Zone::Battlefield);
                assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [3]);
                assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 1);
                assert_eq!(game.player(A).unwrap().library.last(), Some(&untouched));
            }
        }
    }
}
#[test]
fn bag_missing_authenticated_identity_is_incomplete_execution_and_rolls_back_every_prior_resolution_step() {
    for definition in definitions("Bag of Tricks") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let suffix = object(&mut game, A, Zone::Library, "Must stay unexamined", "Mana cost: {3}\nType: Creature — Beast\nPower/Toughness: 3/3");
        let missing = game.create_hidden_card_placeholder(A, Zone::Library, 34, "missing-public-opening".into());
        let index = definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        action(&mut game, LegalAction::ActivateAbility { source, ability_index: index }, &mut Choices::default());
        let paid = game.player(A).unwrap().mana_pool.total();
        let random = game.irreversible_random_count();
        let ids = game.next_object_id_counter();
        game.force_next_die_roll(3);
        let mut dm = ConsultOpenings { pause_at: None, pending: false, opened: Vec::new(), viewed: Vec::new() };
        let error = resolve_stack_entry_with(&mut game, &mut dm).unwrap_err();
        assert!(matches!(error, ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_))));
        assert_eq!(dm.opened, vec![missing]);
        assert!(dm.viewed.is_empty());
        assert_eq!(game.player(A).unwrap().library, vec![suffix, missing]);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), paid);
        assert!(game.is_tapped(source));
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
        assert_eq!(game.irreversible_random_count(), random);
        assert_eq!(game.next_object_id_counter(), ids);
        assert!(game.publicly_revealed_hidden_cards().is_empty());
        assert_eq!(game.take_forced_die_roll(), Some(3));
    }
}
fn prior_reveal_programs(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/declared_any_target_programs.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    program_definitions(name, &format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap()))
}
#[test]
fn prior_explicit_target_reveal_programs_keep_authenticated_match_full_reveal_and_unopened_suffix_distinct() {
    for name in ["Erratic Explosion", "Explosive Revelation"] {
        for definition in prior_reveal_programs(name) {
            for player_target in [false, true] {
                let mut game = game();
                let recipient = object(&mut game, B, Zone::Battlefield, "Saved damage recipient", "Type: Creature — Beast\nPower/Toughness: 2/8");
                let suffix = game.create_hidden_card_placeholder(A, Zone::Library, 35, "private-unexamined-suffix".into());
                let hit = game.create_hidden_card_placeholder(A, Zone::Library, 36, "nonland-hit".into());
                let hit_stable = game.object(hit).unwrap().stable_id;
                let land = game.create_hidden_card_placeholder(A, Zone::Library, 37, "preceding-land".into());
                let hit_definition = compile_to_runtime_definition("Authenticated five", "Mana cost: {5}\nType: Artifact", false).unwrap();
                let land_definition = compile_to_runtime_definition("Authenticated land", "Type: Land", false).unwrap();
                game.reveal_hidden_card_with_definition(hit, &hit_definition).unwrap();
                game.reveal_hidden_card_with_definition(land, &land_definition).unwrap();
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                let target = if player_target { Target::Player(B) } else { Target::Object(recipient) };
                let mut casting = Choices { targets: vec![vec![target]].into(), ..Default::default() };
                cast(&mut game, spell, &mut casting);
                assert_eq!(game.stack.last().unwrap().targets, vec![target]);
                let paid = game.player(A).unwrap().mana_pool.total();
                let mut dm = ConsultOpenings { pause_at: Some(2), pending: false, opened: Vec::new(), viewed: Vec::new() };
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert!(dm.pending);
                assert_eq!(dm.opened, vec![land, hit]);
                assert!(dm.viewed.iter().all(|id| *id == land));
                assert_eq!(game.player(A).unwrap().library, vec![suffix, hit, land]);
                assert_eq!(game.player(B).unwrap().life, 30);
                assert_eq!(game.damage_on(recipient), 0);
                assert_eq!(game.stack.len(), 1);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), paid);
                assert!(game.publicly_revealed_hidden_cards().is_empty());
                dm.pending = false;
                dm.pause_at = None;
                dm.opened.clear();
                dm.viewed.clear();
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.opened, vec![land, hit]);
                assert!(!dm.viewed.contains(&suffix));
                assert!(game.is_hidden_card_placeholder(suffix));
                assert!(!game.is_publicly_revealed_hidden_card(suffix));
                assert_eq!(game.player(B).unwrap().life, if player_target { 25 } else { 30 });
                assert_eq!(game.damage_on(recipient), if player_target { 0 } else { 5 });
                let hit_now = game.find_object_by_stable_id(hit_stable).unwrap();
                assert_eq!(game.object(hit_now).unwrap().zone, if name == "Explosive Revelation" { Zone::Hand } else { Zone::Library });
                assert_eq!(game.object(land).unwrap().zone, Zone::Library);
                assert_eq!(game.player(A).unwrap().library.last(), Some(&suffix));
                assert_eq!(game.player(A).unwrap().library.len(), if name == "Explosive Revelation" { 2 } else { 3 });
            }
        }
    }
}
#[test]
fn prior_explicit_target_reveal_programs_never_treat_an_unopened_placeholder_as_a_nonland_hit() {
    for name in ["Erratic Explosion", "Explosive Revelation"] {
        for definition in prior_reveal_programs(name) {
            let mut game = game();
            let bottom = object(&mut game, A, Zone::Library, "Unexamined hit", "Mana cost: {5}\nType: Artifact");
            let unknown = game.create_hidden_card_placeholder(A, Zone::Library, 38, "unopened-stop-card".into());
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut casting = Choices { targets: vec![vec![Target::Player(B)]].into(), ..Default::default() };
            cast(&mut game, spell, &mut casting);
            let paid = game.player(A).unwrap().mana_pool.total();
            let ids = game.next_object_id_counter();
            let mut dm = ConsultOpenings { pause_at: None, pending: false, opened: Vec::new(), viewed: Vec::new() };
            let error = resolve_stack_entry_with(&mut game, &mut dm).unwrap_err();
            assert!(matches!(error, ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_))));
            assert_eq!(dm.opened, vec![unknown]);
            assert!(dm.viewed.is_empty());
            assert_eq!(game.player(A).unwrap().library, vec![bottom, unknown]);
            assert!(game.player(A).unwrap().hand.is_empty());
            assert_eq!(game.player(B).unwrap().life, 30);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), paid);
            assert_eq!(game.next_object_id_counter(), ids);
            assert!(game.publicly_revealed_hidden_cards().is_empty());
        }
    }
}
#[test]
fn dispatched_consult_keeps_each_reveal_occurrence_in_history_and_commits_each_exactly_once() {
    for definition in program_definitions("Reveal receipt program", "Type: Sorcery\nReveal cards from the top of your library until you reveal an artifact card.") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let untouched = object(&mut game, A, Zone::Library, "Unexposed", "Type: Land");
        let hit = object(&mut game, A, Zone::Library, "Hit", "Type: Artifact");
        let second = object(&mut game, A, Zone::Library, "Second", "Type: Land");
        let first = object(&mut game, A, Zone::Library, "First", "Type: Land");
        let parent = game.alloc_child_event_provenance(Default::default(), ironsmith::events::EventKind::CardRevealed);
        let program = ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(
            definition.spell_effect.as_ref().unwrap().flattened_default_effects().to_vec(),
        ));
        let mut context = EffectContext::new_default(source, A).with_provenance(parent);
        let outcome = ironsmith::effects::execute_effect(&mut game, &program, &mut context).unwrap();
        let reveals = outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).cloned().collect::<Vec<_>>();
        assert_eq!(reveals.iter().map(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().unwrap().card).collect::<Vec<_>>(), vec![first, second, hit]);
        let provenance = reveals.iter().map(|event| event.provenance()).collect::<std::collections::HashSet<_>>();
        assert_eq!(provenance.len(), 3);
        assert!(!provenance.contains(&parent));
        let staged = game.turn_store.turn_history.staged_event_records.iter().filter_map(|record| record.event.downcast::<ironsmith::events::CardRevealedEvent>()).map(|event| event.card).collect::<Vec<_>>();
        assert_eq!(staged, vec![first, second, hit]);
        assert!(!staged.contains(&untouched));
        for _ in 0..2 {
            for event in &reveals { game.queue_trigger_event(Default::default(), event.clone()); }
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
            let committed = game.turn_store.turn_history.event_records.iter().filter_map(|record| record.event.downcast::<ironsmith::events::CardRevealedEvent>()).map(|event| event.card).collect::<Vec<_>>();
            assert_eq!(committed, vec![first, second, hit], "replaying the same observations must not duplicate completed history");
            assert!(game.turn_store.turn_history.staged_event_records.iter().all(|record| record.event.downcast::<ironsmith::events::CardRevealedEvent>().is_none()));
        }
    }
}
#[test]
fn empty_and_all_land_consults_keep_a_known_empty_hit_and_finish_the_complete_damage_program() {
    for name in ["Erratic Explosion", "Explosive Revelation"] {
        for definition in prior_reveal_programs(name) {
            for cards in [0, 3] {
                let mut game = game();
                let mut original = Vec::new();
                for index in 0..cards {
                    original.push(object(&mut game, A, Zone::Library, &format!("Revealed land {index}"), "Type: Land"));
                }
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                let mut dm = Choices { targets: vec![vec![Target::Player(B)]].into(), ..Default::default() };
                cast(&mut game, spell, &mut dm);
                settle(&mut game, &mut dm);
                assert_eq!(game.player(B).unwrap().life, 30);
                assert!(game.player(A).unwrap().hand.is_empty());
                assert_eq!(game.player(A).unwrap().library.len(), cards);
                let remaining = game.player(A).unwrap().library.iter().copied().collect::<std::collections::HashSet<_>>();
                assert_eq!(remaining, original.iter().copied().collect());
                let revealed = game.turn_store.turn_history.event_records.iter()
                    .chain(game.turn_store.turn_history.staged_event_records.iter())
                    .filter_map(|record| record.event.downcast::<ironsmith::events::CardRevealedEvent>())
                    .map(|event| event.card).collect::<Vec<_>>();
                assert_eq!(revealed.len(), cards, "every and only examined card has one physical reveal receipt");
                assert_eq!(revealed.into_iter().collect::<std::collections::HashSet<_>>(), remaining);
                assert!(game.stack.is_empty());
            }
        }
    }
}
#[test]
fn inspiration_preserves_optional_announced_graveyard_targets_and_sums_every_surviving_member() {
    for definition in definitions("Song of Inspiration") {
        for (natural, chosen, remove_high, expected_life) in [(14, 0, false, 30), (20, 0, false, 30), (12, 1, false, 30), (9, 2, false, 37), (20, 2, false, 37), (12, 2, true, 30), (13, 2, true, 32)] {
            let mut game = game();
            let low = object(&mut game, A, Zone::Graveyard, "Two mana card", "Mana cost: {2}\nType: Artifact");
            let high = object(&mut game, A, Zone::Graveyard, "Five mana card", "Mana cost: {5}\nType: Enchantment");
            let untouched = object(&mut game, A, Zone::Graveyard, "Unchosen eleven", "Mana cost: {11}\nType: Creature — Beast\nPower/Toughness: 1/1");
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let targets = [low, high].into_iter().take(chosen).map(Target::Object).collect::<Vec<_>>();
            let mut dm = Choices { targets: vec![targets.clone()].into(), ..Default::default() };
            let mana = game.player(A).unwrap().mana_pool.total();
            cast(&mut game, spell, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 5);
            assert_eq!(game.stack.last().unwrap().targets, targets);
            let later_incarnation = if remove_high {
                let exile = game.move_object_by_effect(high, Zone::Exile).unwrap();
                Some(game.move_object_by_effect(exile, Zone::Graveyard).unwrap())
            } else { None };
            game.force_next_die_roll(natural);
            settle(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().life, expected_life);
            assert_eq!(game.player(A).unwrap().hand.len(), chosen - usize::from(remove_high));
            assert_eq!(game.object(untouched).unwrap().zone, Zone::Graveyard);
            if let Some(later) = later_incarnation { assert_eq!(game.object(later).unwrap().zone, Zone::Graveyard); }
            let mana_sum = if chosen == 0 { 0 } else if chosen == 1 || remove_high { 2 } else { 7 };
            assert_die_receipt(&game, natural, natural + mana_sum);
        }
    }
}
#[test]
fn inspiration_with_all_chosen_targets_illegal_never_rolls_or_gains_life() {
    for definition in definitions("Song of Inspiration") {
        let mut game = game();
        let card = object(&mut game, A, Zone::Graveyard, "Only chosen card", "Mana cost: {5}\nType: Artifact");
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let mut dm = Choices { targets: vec![vec![Target::Object(card)]].into(), ..Default::default() };
        cast(&mut game, spell, &mut dm);
        game.move_object_by_effect(card, Zone::Exile).unwrap();
        game.force_next_die_roll(20);
        settle(&mut game, &mut dm);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 0);
        assert_eq!(game.player(A).unwrap().life, 30);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert_eq!(game.take_forced_die_roll(), Some(20));
    }
}
#[test]
fn two_local_rolls_under_one_program_parent_keep_two_distinct_staged_observations() {
    for definition in program_definitions("Two arithmetic rolls", "Type: Sorcery\nRoll a d20 and add two. Roll a d20 and subtract one.") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let parent = game.alloc_child_event_provenance(Default::default(), ironsmith::events::EventKind::DieRolled);
        let program = ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(
            definition.spell_effect.as_ref().unwrap().flattened_default_effects().to_vec(),
        ));
        game.force_next_die_roll(3);
        game.force_next_die_roll(9);
        let mut context = EffectContext::new_default(source, A).with_provenance(parent);
        let outcome = ironsmith::effects::execute_effect(&mut game, &program, &mut context).unwrap();
        let events = outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::other::DieRolledEvent>().is_some()).collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].provenance(), events[1].provenance());
        let staged = game.turn_store.turn_history.staged_event_records.iter().filter_map(|record| record.event.downcast::<ironsmith::events::other::DieRolledEvent>()).map(|event| (event.natural_result, event.result)).collect::<Vec<_>>();
        assert_eq!(staged, vec![(3, 5), (9, 8)]);
        assert_eq!(game.turn_store.turn_history.die_rolls_this_turn[&A], [5, 8]);
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), 2);
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        let committed = game.turn_store.turn_history.event_records.iter().filter_map(|record| record.event.downcast::<ironsmith::events::other::DieRolledEvent>()).map(|event| (event.natural_result, event.result)).collect::<Vec<_>>();
        assert_eq!(committed, vec![(3, 5), (9, 8)]);
    }
}

#[path = "die_result_programs/numeric_owner.rs"]
mod numeric_owner;

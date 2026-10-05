//! Full native scenarios, authored but not executed under the campaign policy.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, NumberContext, OrderContext, SelectObjectsContext, SelectOptionsContext, TargetsContext, ViewCardsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const COMPLETE: &[&str] = &["Atris, Oracle of Half-Truths", "Curator of Destinies", "Epiphany at the Drownyard", "Fortune's Favor", "Riddles in the Dark", "Steam Augury", "Truth or Tale"];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/binary_card_piles.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text += &format!("Power/Toughness: {p}/{t}\n");
    }
    text += row["oracle_text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(artifact, decoded);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [&direct, &restored] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        let rendered = ironsmith_text::canonical_compiled_lines(definition).join("\n");
        assert!(rendered.contains("pile"), "{name}: {rendered}");
        assert!(!rendered.contains("tagged-object") && !rendered.contains("binary_") && !rendered.contains("__it__"), "{name}: {rendered}");
    }
    [direct, restored]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) -> Vec<ObjectId> {
    let definition = compile_to_runtime_definition("Library card", "Type: Artifact", false).unwrap();
    (0..count).map(|_| game.create_object_from_definition(&definition, owner, Zone::Library)).collect()
}
struct Choices {
    pool: Vec<ObjectId>, partitioner: PlayerId, chooser: PlayerId,
    take: usize, mode: usize, x: u32,
    selected: Vec<ObjectId>, single: Option<ObjectId>, ordered: Vec<ObjectId>,
    views: Vec<(PlayerId, bool, Vec<ObjectId>)>, targets: usize,
    pause_partition: bool, pause_replacement: bool, pending: bool,
}
impl Choices {
    fn new(pool: Vec<ObjectId>, targeted: bool, take: usize, mode: usize) -> Self {
        Self { pool, partitioner: if targeted { C } else { A }, chooser: if targeted { A } else { C },
            take, mode, x: 4, selected: vec![], single: None, ordered: vec![], views: vec![], targets: 0,
            pause_partition: false, pause_replacement: false, pending: false }
    }
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.pending = self.pause_replacement;
        true
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&Target::Player(C)));
        assert!(!context.requirements[0].legal_targets.contains(&Target::Player(A)));
        self.targets += 1;
        vec![Target::Player(C)]
    }
    fn decide_objects(&mut self, _: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let candidates: Vec<_> = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        assert!(candidates.iter().all(|id| self.pool.contains(id)), "finite source set");
        if context.reveal_policy == ironsmith::decisions::context::SelectionRevealPolicy::Public {
            return candidates;
        }
        if context.min == 0 {
            assert_eq!(context.player, self.partitioner);
            assert_eq!(candidates.len(), self.pool.len());
            assert_eq!(context.max, Some(self.pool.len()));
            self.selected = candidates.into_iter().take(self.take).collect();
            self.pending = self.pause_partition;
            self.selected.clone()
        } else {
            assert_eq!(context.player, A);
            assert_eq!(context.min, 1);
            assert_eq!(context.max, Some(1));
            let chosen = candidates[0];
            assert_eq!(self.selected.contains(&chosen), self.mode == 0);
            self.single = Some(chosen);
            vec![chosen]
        }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(cara) = context.options.iter().find(|option| option.description == "Cara") {
            assert_eq!(context.player, A);
            return vec![cara.index];
        }
        if context.options.iter().any(|option| option.description.starts_with("Choose pile") || option.description.starts_with("Choose the face-")) {
            assert_eq!(context.player, self.chooser);
            assert_eq!(context.options.len(), 2, "empty piles remain legal alternatives");
            assert!(context.options.iter().all(|option| option.legal));
            for (index, option) in context.options.iter().enumerate() {
                let shown = option.related_object_ids.as_ref().expect("each choice identifies its exact pile");
                let expected: Vec<_> = self.pool.iter().filter(|id| self.selected.contains(id) == (index == 0)).copied().collect();
                assert_eq!(shown.len(), expected.len());
                assert!(shown.iter().all(|id| expected.contains(id)));
            }
            return vec![self.mode];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        assert!(self.x >= context.min && self.x <= context.max);
        self.x
    }
    fn decide_order(&mut self, _: &GameState, context: &OrderContext) -> Vec<ObjectId> {
        assert_eq!(context.player, A);
        assert!(context.items.iter().all(|(id, _)| self.pool.contains(id)));
        self.ordered = context.items.iter().rev().map(|(id, _)| *id).collect();
        self.ordered.clone()
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], context: &ViewCardsContext) {
        assert_eq!(context.subject, A);
        assert_eq!(context.zone, Zone::Library);
        assert!(cards.iter().all(|id| self.pool.contains(id)), "never inspect unrelated library cards");
        self.views.push((viewer, context.public, cards.to_vec()));
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn cast(game: &mut GameState, definition: &CardDefinition, choices: &mut impl DecisionMaker) -> ObjectId {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand, casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        }), choices).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(state.pending_cast.is_none());
    assert_eq!(game.stack.len(), 1);
    game.stack[0].object_id
}

#[path = "binary_card_piles/sequential_exile.rs"]
mod sequential_exile;
#[path = "binary_card_piles/arrival_counters.rs"]
mod arrival_counters;
fn resolve(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..16 {
        if game.stack.is_empty() || choices.pending { return; }
        resolve_stack_entry_with(game, choices).unwrap();
        if !choices.pending {
            put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
        }
    }
    panic!("stack did not settle");
}
fn targeted(name: &str) -> bool { name == "Fortune's Favor" || name.starts_with("Atris,") }
fn count(name: &str) -> usize {
    match name { "Atris, Oracle of Half-Truths" => 3, "Fortune's Favor" | "Riddles in the Dark" => 4, _ => 5 }
}

#[test]
fn seven_full_bodies_partition_every_legal_size_and_preserve_untouched_cards() {
    for &name in COMPLETE {
        for definition in definitions(name) {
            for available in [0, 1, count(name), count(name) + 2] {
                for take in 0..=available.min(count(name)) {
                    for mode in 0..2 {
                        let mut game = game();
                        let all = library(&mut game, A, available);
                        let foreign = library(&mut game, B, 3);
                        let pool: Vec<_> = all.iter().rev().take(count(name)).copied().collect();
                        let stable: Vec<_> = pool.iter().map(|id| (*id, game.object(*id).unwrap().stable_id)).collect();
                        let mut choices = Choices::new(pool.clone(), targeted(name), take, mode);
                        let spell = cast(&mut game, &definition, &mut choices);
                        if name == "Curator of Destinies" { assert!(!game.can_be_countered(spell)); }
                        assert_eq!(game.stack[0].targets.len(), usize::from(name == "Fortune's Favor"));
                        resolve(&mut game, &mut choices);
                        assert_eq!(choices.targets, usize::from(targeted(name)));
                        for &(old, identity) in &stable {
                            let current = game.find_object_by_stable_id(identity).unwrap();
                            let expected = if name == "Truth or Tale" {
                                if choices.single == Some(old) { Zone::Hand } else { Zone::Library }
                            } else if choices.selected.contains(&old) == (mode == 0) { Zone::Hand } else { Zone::Graveyard };
                            assert_eq!(game.object(current).unwrap().zone, expected, "{name}, take {take}, mode {mode}");
                        }
                        let unchanged: Vec<_> = all.iter().filter(|id| !pool.contains(id)).copied().collect();
                        let final_library = &game.player(A).unwrap().library;
                        if name == "Truth or Tale" {
                            assert!(final_library.ends_with(&unchanged));
                            if choices.ordered.len() > 1 {
                                let identities: Vec<_> = final_library.iter().take(choices.ordered.len())
                                    .map(|id| game.object(*id).unwrap().stable_id).collect();
                                let expected: Vec<_> = choices.ordered.iter()
                                    .map(|old| stable.iter().find(|(id, _)| id == old).unwrap().1).collect();
                                assert_eq!(identities, expected);
                            }
                        } else { assert_eq!(final_library, &unchanged); }
                        assert_eq!(game.player(B).unwrap().library, foreign);
                        let hidden = matches!(name, "Atris, Oracle of Half-Truths" | "Fortune's Favor" | "Curator of Destinies" | "Riddles in the Dark");
                        for (viewer, public, cards) in &choices.views {
                            if !public { assert!(hidden); assert_eq!(*viewer, choices.partitioner); }
                            if *public && hidden { assert!(cards.iter().all(|id| !choices.selected.contains(id)), "face-down pile stays private"); }
                        }
                        if !pool.is_empty() { assert!(!choices.views.is_empty()); }
                    }
                }
            }
        }
    }
}

#[test]
fn epiphany_announces_x_and_reveals_x_plus_one_even_for_zero() {
    for definition in definitions("Epiphany at the Drownyard") {
        for x in [0, 2, 6] {
            let mut game = game();
            let all = library(&mut game, A, 9);
            let pool = all.iter().rev().take(x as usize + 1).copied().collect();
            let mut choices = Choices::new(pool, false, 0, 0);
            choices.x = x;
            cast(&mut game, &definition, &mut choices);
            assert_eq!(game.object(game.stack[0].object_id).unwrap().x_value, Some(x));
            resolve(&mut game, &mut choices);
            assert_eq!(game.player(A).unwrap().library.len(), 8 - x as usize);
            assert!(game.player(A).unwrap().hand.is_empty());
        }
    }
}

#[test]
fn pending_partition_restores_native_resolution_and_replays_the_same_pool() {
    for definition in definitions("Fortune's Favor") {
        let mut game = game();
        let all = library(&mut game, A, 6);
        let pool = all.iter().rev().take(4).copied().collect();
        let mut choices = Choices::new(pool, true, 2, 0);
        choices.pause_partition = true;
        cast(&mut game, &definition, &mut choices);
        resolve(&mut game, &mut choices);
        assert!(choices.pending);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().library, all);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.player(A).unwrap().graveyard.is_empty());
        choices.pending = false;
        choices.pause_partition = false;
        choices.views.clear();
        resolve(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(A).unwrap().library.len(), 2);
    }
}

#[test]
fn both_original_pile_destinations_complete_before_any_replacement_added_program() {
    for definition in definitions("Steam Augury") {
        for blink_back in [false, true] {
            let mut game = game();
            let all = library(&mut game, A, 5);
            let identities: Vec<_> = all.iter().map(|id| game.object(*id).unwrap().stable_id).collect();
            let mut choices = Choices::new(all, false, 2, 0);
            let source = cast(&mut game, &definition, &mut choices);
            let own_library = ironsmith::target::ObjectFilter::default().in_zone(Zone::Library)
                .owned_by(ironsmith::target::PlayerFilter::You);
            let graveyard = ironsmith::target::ObjectFilter::default().in_zone(Zone::Graveyard)
                .owned_by(ironsmith::target::PlayerFilter::You);
            let mut additions = vec![
                ironsmith::effect::Effect::gain_life(ironsmith::effect::Value::Count(graveyard.clone())),
                ironsmith::effect::Effect::move_to_zone(
                    ironsmith::target::ChooseSpec::All(graveyard), Zone::Exile, false),
            ];
            if blink_back {
                additions.push(ironsmith::effect::Effect::move_to_zone(
                    ironsmith::target::ChooseSpec::All(ironsmith::target::ObjectFilter::default()
                        .in_zone(Zone::Exile).owned_by(ironsmith::target::PlayerFilter::You)), Zone::Library, false));
            }
            game.effect_store.replacement_effects.add_one_shot_effect(
                ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                        own_library, Some(Zone::Library), Some(Zone::Hand)),
                    ironsmith::replacement::ReplacementAction::Additionally(additions)));
            resolve(&mut game, &mut choices);
            assert_eq!(game.player(A).unwrap().life, 23, "the added program observes all three original graveyard arrivals");
            assert_eq!(game.player(A).unwrap().hand.len(), 2);
            for identity in identities {
                let current = game.find_object_by_stable_id(identity).unwrap();
                assert_ne!(game.object(current).unwrap().zone, Zone::Graveyard,
                    "the captured unchosen pile does not follow a replacement-created incarnation");
            }
        }
    }
}

#[test]
fn truth_preserves_the_selected_card_when_its_hand_move_is_prevented() {
    for definition in definitions("Truth or Tale") {
        let mut game = game();
        let all = library(&mut game, A, 5);
        let mut choices = Choices::new(all.clone(), false, 2, 0);
        let source = cast(&mut game, &definition, &mut choices);
        game.effect_store.replacement_effects.add_resolution_effect(
            ironsmith::replacement::ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ironsmith::target::ObjectFilter::default(), Some(Zone::Library), Some(Zone::Hand)),
                ironsmith::replacement::ReplacementAction::Prevent));
        resolve(&mut game, &mut choices);
        assert!(game.player(A).unwrap().hand.is_empty());
        let selected = choices.single.unwrap();
        assert_eq!(game.player(A).unwrap().library.last(), Some(&selected));
        assert_eq!(choices.ordered.len(), 4);
        assert!(!choices.ordered.contains(&selected));
    }
}

#[test]
fn unsupported_tails_cannot_succeed_by_accepting_only_the_pile_prefix() {
    let base = "Type: Instant\nReveal the top five cards of your library and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.";
    for bad in [
        format!("{base} Frobulate the chosen pile."),
        base.replace("two piles", "three piles"),
        base.replace("two piles", "two piles at random"),
        base.replace("five cards", "five {U} cards"),
    ] {
        let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Unsupported pile program", &bad, false));
        assert!(direct.is_err() || loss.is_lossy(), "{bad}");
        let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Unsupported pile program", &bad, false));
        assert!(artifact.is_err() || loss.is_lossy(), "{bad}");
    }
}

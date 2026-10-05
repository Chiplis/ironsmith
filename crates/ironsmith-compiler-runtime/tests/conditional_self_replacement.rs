//! Frozen full bodies. Authored, source-reviewed, and deliberately UNRUN.
use ironsmith::card::CardBuilder;
use ironsmith::cards::CardDefinition;
use ironsmith::cost::OptionalCostsPaid;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
    apply_decision_context_with_dm, extract_target_requirements_from_program_with_modes, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, StackEntry, TargetAssignment};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/conditional_self_replacement.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, types: Vec<CardType>) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Quantity fixture").card_types(types).build(), owner, zone)
}
fn hand(game: &mut GameState, owner: PlayerId, count: usize) {
    for _ in 0..count { card(game, owner, Zone::Hand, vec![CardType::Land]); }
}
fn mana(game: &mut GameState, player: PlayerId, color: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(color, amount);
}
#[derive(Default)]
struct Choices {
    kick: bool, target: Option<Target>, lands: Vec<ObjectId>, discard_actors: Vec<PlayerId>,
    pause_discard: bool, pending: bool,
    pay: bool, payers: Vec<PlayerId>, pause_payment: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.payers.push(ctx.player);
        if self.pause_payment { self.pending = true; return false; }
        self.pay
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            assert_eq!(ctx.player, A);
            return if self.kick { vec![ctx.options.iter().find(|option| option.legal).expect("complete kicker can be paid").index] } else { vec![] };
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let target = self.target.unwrap();
        assert_eq!(ctx.requirements.len(), 1, "replacement uses the original declaration");
        assert!(ctx.requirements[0].legal_targets.contains(&target)); vec![target]
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if !self.lands.is_empty() && self.lands.iter().all(|id| ctx.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal)) {
            assert_eq!(ctx.player, A); assert_eq!(ctx.min, 2); return self.lands.clone();
        }
        if ctx.candidates.iter().any(|candidate| game.object(candidate.id).is_some_and(|object| object.zone == Zone::Hand)) {
            self.discard_actors.push(ctx.player);
            if self.pause_discard { self.pending = true; return vec![]; }
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

fn graveyard_count(game: &mut GameState, owner: PlayerId, count: usize) {
    for _ in 0..count { card(game, owner, Zone::Graveyard, vec![CardType::Instant]); }
}

#[test]
fn epicenter_retains_its_target_and_rechecks_threshold_before_one_sacrifice_program() {
    for definition in definitions("Epicenter") { for threshold in [false, true] {
        let mut game = game();
        let a = card(&mut game, A, Zone::Battlefield, vec![CardType::Land]);
        let b = card(&mut game, B, Zone::Battlefield, vec![CardType::Land]);
        let b2 = card(&mut game, B, Zone::Battlefield, vec![CardType::Land]);
        let c = card(&mut game, C, Zone::Battlefield, vec![CardType::Land]);
        let foreign_owned = card(&mut game, B, Zone::Battlefield, vec![CardType::Land]);
        game.set_current_controller(foreign_owned, A).unwrap();
        let nonland = card(&mut game, B, Zone::Battlefield, vec![CardType::Artifact]);
        let outside = card(&mut game, A, Zone::Hand, vec![CardType::Land]);
        graveyard_count(&mut game, B, 9);
        if !threshold { graveyard_count(&mut game, A, 7); }
        mana(&mut game, A, ManaSymbol::Red, 1); mana(&mut game, A, ManaSymbol::Colorless, 4);
        let mut dm = Choices { target: Some(Target::Player(B)), ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.stack.last().unwrap().targets, vec![Target::Player(B)]);
        assert_eq!(game.stack.last().unwrap().target_assignments.len(), 1);
        if threshold { graveyard_count(&mut game, A, 7); }
        else { for id in game.player(A).unwrap().graveyard.clone() { game.move_object_by_effect(id, Zone::Exile).unwrap(); } }
        let lands = [a, b, b2, c, foreign_owned];
        let identities = lands.map(|id| game.object(id).unwrap().stable_id);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.object(nonland).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(outside).unwrap().zone, Zone::Hand);
        let moved = identities.iter().filter(|stable| game.object(game.find_object_by_stable_id(**stable).unwrap()).unwrap().zone == Zone::Graveyard).count();
        assert_eq!(moved, if threshold { 5 } else { 1 });
        if !threshold {
            for id in [a, c, foreign_owned] { assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield); }
            assert_eq!([b, b2].iter().filter(|id| game.object(**id).is_some_and(|object| object.zone == Zone::Battlefield)).count(), 1);
        }
    }}
}

#[test]
fn epicenter_copy_rechecks_threshold_and_an_illegal_original_target_stops_everyone() {
    for definition in definitions("Epicenter") { for illegal in [false, true] {
        let mut game = game();
        for owner in [A, B, C] { for _ in 0..3 { card(&mut game, owner, Zone::Battlefield, vec![CardType::Land]); } }
        let spell = put_spell(&mut game, &definition, Target::Player(B), OptionalCostsPaid::default());
        if illegal {
            graveyard_count(&mut game, A, 7); game.player_mut(B).unwrap().has_lost = true;
            resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
            assert_eq!(game.battlefield.len(), 9, "the printed target remains required even in the all-player branch");
        } else {
            copy(&mut game, spell);
            resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
            assert_eq!(game.battlefield.len(), 8);
            graveyard_count(&mut game, A, 7);
            resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
            assert!(game.battlefield.is_empty());
        }
    }}
}

#[test]
fn epicenter_late_replacement_suspension_or_resource_error_restores_the_whole_batch() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Epicenter") { for resource_failure in [false, true] {
        let mut game = game(); graveyard_count(&mut game, A, 7);
        let lands = [card(&mut game, A, Zone::Battlefield, vec![CardType::Land]),
            card(&mut game, B, Zone::Battlefield, vec![CardType::Land]),
            card(&mut game, C, Zone::Battlefield, vec![CardType::Land])];
        let spell = put_spell(&mut game, &definition, Target::Player(B), OptionalCostsPaid::default());
        let addition = if resource_failure {
            Effect::new(ironsmith::effects::CreateTokenEffect::you(ironsmith::cards::tokens::treasure_token_definition(), 1))
        } else { Effect::may(vec![Effect::gain_life(2)]) };
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(spell, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(lands[2]), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            ReplacementAction::Instead(vec![Effect::gain_life(1), addition])));
        let mut dm = Choices { pause_payment: !resource_failure, ..Default::default() };
        if resource_failure { game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 0, ..Default::default() }); }
        let ids = game.next_object_id_counter();
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert_eq!(result.is_err(), resource_failure);
        if !resource_failure { assert!(dm.pending); }
        for id in lands { assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield); }
        assert_eq!(game.stack.len(), 1); assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), ids);
        game.set_token_creation_limits(Default::default()); dm.pause_payment = false; dm.pending = false;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.object(lands[0]).is_none()); assert!(game.object(lands[1]).is_none());
        assert_eq!(game.object(lands[2]).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().life, 21);
        assert!(game.stack.is_empty());
    }}
}

#[test]
fn epicenter_prepares_replacements_before_any_original_and_finishes_additions_after_all() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    for definition in definitions("Epicenter") {
        let mut game = game(); graveyard_count(&mut game, A, 7);
        let lord = compile_to_runtime_definition("Simultaneous land witness",
            "Type: Land Creature\nPower/Toughness: 1/3\nOther creatures get +1/+1.", false).unwrap();
        let body = compile_to_runtime_definition("Simultaneous affected land", "Type: Land Creature\nPower/Toughness: 1/3", false).unwrap();
        let a = game.create_object_from_definition(&lord, A, Zone::Battlefield);
        let b = game.create_object_from_definition(&body, B, Zone::Battlefield);
        let c = game.create_object_from_definition(&body, C, Zone::Battlefield);
        assert_eq!(game.current_power(b), Some(2));
        let b_stable = game.object(b).unwrap().stable_id;
        let c_stable = game.object(c).unwrap().stable_id;
        let spell = put_spell(&mut game, &definition, Target::Player(B), OptionalCostsPaid::default());
        card(&mut game, A, Zone::Library, vec![CardType::Land]);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(spell, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(a), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            ReplacementAction::Additionally(vec![Effect::draw(1), Effect::gain_life(ironsmith::effect::Value::Count(ObjectFilter::land().controlled_by(PlayerFilter::Opponent)))])));
        let mut b_filter = ObjectFilter::specific(b); b_filter.power = Some(ironsmith::target::Comparison::GreaterThanOrEqual(2));
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(spell, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(b_filter, Some(Zone::Battlefield), Some(Zone::Graveyard)),
            ReplacementAction::ChangeDestination(Zone::Exile)));
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(b_stable).unwrap()).unwrap().zone, Zone::Exile,
            "B's replacement matched while A's continuous effect still existed");
        assert_eq!(game.object(game.find_object_by_stable_id(c_stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.player(A).unwrap().life, 20, "A's addition observes both later players' completed original departures");
        assert_eq!(game.player(A).unwrap().hand.len(), 1, "the replacement-added native draw completes once after all originals");
        assert!(game.battlefield.is_empty());
    }
}

fn opposing_spell(game: &mut GameState, uncounterable: bool) -> ObjectId {
    let text = if uncounterable { "Type: Instant\nThis spell can't be countered.\nYou gain 1 life." }
        else { "Type: Instant\nYou gain 1 life." };
    let definition = compile_to_runtime_definition("Counter target fixture", text, false).unwrap();
    let spell = game.create_object_from_definition(&definition, B, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, B)); spell
}

#[test]
fn bring_the_ending_rechecks_target_controller_poison_and_preserves_the_optional_payment() {
    for definition in definitions("Bring the Ending") { for (poison, pay, available, uncounterable) in [
        (0, false, 2, false), (0, true, 2, false), (0, true, 0, false),
        (3, false, 2, false), (3, true, 2, false), (0, true, 2, true), (3, true, 2, true),
    ] {
        let mut game = game(); let target = opposing_spell(&mut game, uncounterable);
        game.player_mut(A).unwrap().poison_counters = 4; game.player_mut(C).unwrap().poison_counters = 4;
        game.player_mut(B).unwrap().poison_counters = if poison == 0 { 3 } else { 0 };
        mana(&mut game, A, ManaSymbol::Blue, 1); mana(&mut game, A, ManaSymbol::Colorless, 1);
        mana(&mut game, B, ManaSymbol::Colorless, available);
        let mut dm = Choices { target: Some(Target::Object(target)), pay, ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.player_mut(B).unwrap().poison_counters = poison;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let paid = poison == 0 && available >= 2 && pay;
        let survives = uncounterable || paid;
        assert_eq!(game.stack.iter().any(|entry| entry.object_id == target), survives);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), available - if paid { 2 } else { 0 });
        assert_eq!(dm.payers, if poison == 0 && available >= 2 { vec![B] } else { vec![] });
    }}
}

#[test]
fn bring_the_ending_uses_new_target_controller_and_retained_original_target_on_copy() {
    for definition in definitions("Bring the Ending") { for poison in [0, 3] {
        let mut game = game(); let target = opposing_spell(&mut game, false);
        game.player_mut(B).unwrap().poison_counters = if poison == 0 { 3 } else { 0 };
        mana(&mut game, C, ManaSymbol::Colorless, 4);
        let spell = put_spell(&mut game, &definition, Target::Object(target), OptionalCostsPaid::default());
        copy(&mut game, spell);
        game.set_current_controller(target, C).unwrap(); assert_eq!(game.current_controller(target), Some(C));
        game.player_mut(C).unwrap().poison_counters = poison;
        let mut dm = Choices { pay: true, ..Default::default() };
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.payers, if poison == 0 { vec![C, C] } else { vec![] });
        assert_eq!(game.stack.iter().any(|entry| entry.object_id == target), poison == 0);
        assert_eq!(game.player(C).unwrap().mana_pool.total(), if poison == 0 { 0 } else { 4 });
    }}
}

#[test]
fn bring_the_ending_pending_payment_and_resource_error_restore_spell_target_and_mana() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Bring the Ending") { for resource_failure in [false, true] {
        let mut game = game(); let target = opposing_spell(&mut game, false);
        mana(&mut game, B, ManaSymbol::Colorless, 2);
        let spell = put_spell(&mut game, &definition, Target::Object(target), OptionalCostsPaid::default());
        if resource_failure {
            game.player_mut(B).unwrap().poison_counters = 3;
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(spell, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(target), Some(Zone::Stack), Some(Zone::Graveyard)),
                ReplacementAction::Additionally(vec![Effect::gain_life(1), Effect::new(ironsmith::effects::CreateTokenEffect::you(ironsmith::cards::tokens::treasure_token_definition(), 1))])));
            game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 0, ..Default::default() });
        }
        let mut dm = Choices { pause_payment: !resource_failure, pay: true, ..Default::default() };
        let result = resolve_stack_entry_with(&mut game, &mut dm);
        assert_eq!(result.is_err(), resource_failure);
        assert_eq!(game.stack.len(), 2); assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 2); assert_eq!(game.player(A).unwrap().life, 20);
        game.set_token_creation_limits(Default::default()); dm.pause_payment = false; dm.pending = false;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.stack.iter().any(|entry| entry.object_id == target), !resource_failure);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), if resource_failure { 2 } else { 0 });
    }}
}

#[test]
fn trailing_replacement_clauses_reject_embedded_symbols_and_unconsumed_suffixes() {
    for text in [
        "Target player sacrifices a land of their choice. Each player sacrifices all lands {R} they control instead if there are seven or more cards in your graveyard.",
        "Target player sacrifices a land of their choice. Each player sacrifices all lands they control instead if there are seven or more cards in your graveyard {R}.",
        "Counter target spell unless its controller pays {2}. Counter that spell {U} instead if its controller has three or more poison counters.",
        "Counter target spell unless its controller pays {2}. Counter that spell instead if its controller has three or more poison counters:.",
        "Counter target spell unless its controller pays {2}. Counter that spell instead if its controller has three or more poison counters instead.",
    ] {
        let text = format!("Type: Instant\n{text}");
        assert!(compile_to_runtime_definition("Malformed trailing replacement", &text, false).is_err(), "{text}");
        assert!(compile_to_artifact("Malformed trailing replacement", &text, false).is_err(), "{text}");
    }
}

#[test]
fn a_shared_controller_poison_condition_uses_its_non_target_antecedent() {
    let text = "Type: Instant\nChoose target creature. Choose a creature you control. If its controller has three or more poison counters, you gain 3 life.";
    let direct = compile_to_runtime_definition("Controller antecedent fixture", text, false).unwrap();
    let (artifact, _) = compile_to_artifact("Controller antecedent fixture", text, false).unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [direct, restored] { for chosen_controller_poisoned in [false, true] {
        let mut game = game();
        let creature = CardBuilder::new(CardId::new(), "Antecedent creature").card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        game.create_object_from_card(&creature, A, Zone::Battlefield);
        let target = game.create_object_from_card(&creature, B, Zone::Battlefield);
        game.player_mut(A).unwrap().poison_counters = if chosen_controller_poisoned { 3 } else { 0 };
        game.player_mut(B).unwrap().poison_counters = if chosen_controller_poisoned { 0 } else { 3 };
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let requirements = extract_target_requirements_from_program_with_modes(&game, definition.spell_effect.as_ref().unwrap(), A, Some(source), None);
        assert_eq!(requirements.len(), 1);
        game.push_to_stack(StackEntry::new(source, A).with_targets(vec![Target::Object(target)])
            .with_target_assignments(vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 }]));
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.player(A).unwrap().life, if chosen_controller_poisoned { 23 } else { 20 },
            "the unrelated explicit target does not replace the last chosen object");
    }}
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("printed mana and optional cost are legal");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { return game.stack.last().unwrap().object_id; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        *game = game.clone(); state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast did not finish");
}
fn put_spell(game: &mut GameState, definition: &CardDefinition, target: Target, paid: OptionalCostsPaid) -> ObjectId {
    let spell = game.create_object_from_definition(definition, A, Zone::Stack);
    game.object_mut(spell).unwrap().optional_costs_paid = paid.clone();
    let program = definition.spell_effect.as_ref().unwrap();
    assert_eq!(program.segments.len(), 1, "one default action with one replacement");
    assert_eq!(program.segments[0].self_replacements.len(), 1);
    let requirements = extract_target_requirements_from_program_with_modes(game, program, A, Some(spell), None);
    assert_eq!(requirements.len(), 1, "one fixed original target even when everyone is affected");
    assert_eq!(requirements[0].min_targets, 1);
    game.push_to_stack(StackEntry::new(spell, A).with_optional_costs_paid(paid).with_targets(vec![target])
        .with_target_assignments(vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 }]));
    spell
}
fn copy(game: &mut GameState, spell: ObjectId) {
    execute_effect(game, &Effect::new(ironsmith::effects::CopySpellEffect::single(ChooseSpec::SpecificObject(spell))),
        &mut EffectContext::new(spell, A, &mut SelectFirstDecisionMaker)).unwrap();
}

#[test]
fn every_frozen_body_has_one_real_replacement_and_retains_its_printed_semantic_markers() {
    for name in ["Bog Down", "Hypnotic Cloud", "Haunting Hymn", "Whispers of Emrakul", "Epicenter", "Bring the Ending"] {
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let program = definition.spell_effect.as_ref().unwrap();
            assert_eq!(program.segments.len(), 1, "{name}");
            assert_eq!(program.segments[0].self_replacements.len(), 1, "{name}");
            assert!(!program.segments[0].default_effects.is_empty());
            assert!(!program.segments[0].self_replacements[0].replacement_effects.is_empty());
            let text = ironsmith_text::canonical_compiled_lines(&definition).join("\n").to_lowercase();
            assert!(text.contains("instead"), "{name}: {text}");
            for marker in match name {
                "Bog Down" => vec!["kicker", "two lands", "three cards"],
                "Hypnotic Cloud" => vec!["kicker", "{4}", "three cards"],
                "Haunting Hymn" => vec!["main phase", "four cards"],
                "Whispers of Emrakul" => vec!["opponent", "at random", "card types", "graveyard"],
                "Epicenter" => vec!["target player", "each player", "all lands", "graveyard"],
                _ => vec!["unless", "{2}", "poison counters"],
            } { assert!(text.contains(marker), "{name}: {marker}: {text}"); }
        }
    }
}

#[test]
fn kicked_discard_preserves_full_cost_choices_and_copies_without_paying_twice() {
    for name in ["Bog Down", "Hypnotic Cloud"] { for definition in definitions(name) { for kicked in [false, true] {
        let mut game = game(); hand(&mut game, B, 8); hand(&mut game, C, 4);
        let lands = [card(&mut game, A, Zone::Battlefield, vec![CardType::Land]), card(&mut game, A, Zone::Battlefield, vec![CardType::Land])];
        let foreign = card(&mut game, B, Zone::Battlefield, vec![CardType::Land]);
        let generic = if name == "Bog Down" { 2 } else { 1 + if kicked { 4 } else { 0 } };
        mana(&mut game, A, ManaSymbol::Black, 1); mana(&mut game, A, ManaSymbol::Colorless, generic);
        let mut dm = Choices { kick: kicked, target: Some(Target::Player(B)), lands: lands.to_vec(), ..Default::default() };
        let spell = cast(&mut game, &definition, &mut dm);
        assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_kicked(), kicked);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.player(A).unwrap().graveyard.len(), if name == "Bog Down" && kicked { 2 } else { 0 });
        copy(&mut game, spell); assert_eq!(game.stack.len(), 2);
        assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_kicked(), kicked);
        game = game.clone();
        for _ in 0..2 { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        let amount = if kicked { 3 } else if name == "Bog Down" { 2 } else { 1 };
        assert_eq!(game.player(B).unwrap().hand.len(), 8 - 2 * amount);
        assert_eq!(game.player(C).unwrap().hand.len(), 4);
        assert!(dm.discard_actors.iter().all(|actor| *actor == B));
    }}}
}

#[test]
fn bog_down_modified_sacrifice_payment_still_records_the_kicker_choice() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Bog Down") {
        let mut game = game(); hand(&mut game, B, 5);
        let lands = [card(&mut game, A, Zone::Battlefield, vec![CardType::Land]), card(&mut game, A, Zone::Battlefield, vec![CardType::Land])];
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(lands[0], A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(lands[0]), Some(Zone::Battlefield), Some(Zone::Graveyard)), ReplacementAction::Prevent));
        mana(&mut game, A, ManaSymbol::Black, 1); mana(&mut game, A, ManaSymbol::Colorless, 2);
        let mut dm = Choices { kick: true, target: Some(Target::Player(B)), lands: lands.to_vec(), ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert!(game.stack.last().unwrap().optional_costs_paid.was_kicked());
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1, "one original sacrifice was prevented");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), 2, "the complete modified cost was paid; the kicker choice is preserved");
    }
}

#[test]
fn haunting_hymn_reads_the_cast_phase_and_does_not_copy_an_actual_cast_event() {
    for definition in definitions("Haunting Hymn") { for (active, phase, main) in [
        (A, Phase::FirstMain, true), (A, Phase::NextMain, true), (B, Phase::FirstMain, false), (A, Phase::Beginning, false),
    ] {
        let mut game = game(); game.turn.active_player = active; game.turn.phase = phase;
        hand(&mut game, B, 9); mana(&mut game, A, ManaSymbol::Black, 2); mana(&mut game, A, ManaSymbol::Colorless, 4);
        let mut dm = Choices { target: Some(Target::Player(B)), ..Default::default() };
        let spell = cast(&mut game, &definition, &mut dm);
        game.turn.active_player = A; game.turn.phase = if main { Phase::Beginning } else { Phase::FirstMain };
        copy(&mut game, spell);
        assert!(!game.stack.last().unwrap().optional_costs_paid.was_paid_label("CastDuringYourMainPhase"));
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.player(B).unwrap().hand.len(), 7);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert_eq!(game.player(B).unwrap().hand.len(), if main { 3 } else { 5 });
    }}
}

#[test]
fn whispers_rechecks_distinct_card_types_in_its_controllers_current_graveyard() {
    for definition in definitions("Whispers of Emrakul") { for threshold in [false, true] {
        let mut game = game(); hand(&mut game, B, 5); hand(&mut game, C, 5);
        for kind in [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Instant] {
            card(&mut game, B, Zone::Graveyard, vec![kind]);
        }
        card(&mut game, A, Zone::Graveyard, vec![CardType::Artifact, CardType::Creature]);
        let changing = card(&mut game, A, if threshold { Zone::Hand } else { Zone::Graveyard }, vec![CardType::Enchantment, CardType::Instant]);
        let spell = put_spell(&mut game, &definition, Target::Player(B), OptionalCostsPaid::default());
        game.move_object_by_effect(changing, if threshold { Zone::Graveyard } else { Zone::Exile }).unwrap();
        copy(&mut game, spell);
        let mut dm = Choices::default();
        for _ in 0..2 { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert_eq!(game.player(B).unwrap().hand.len(), if threshold { 1 } else { 3 });
        assert_eq!(game.player(C).unwrap().hand.len(), 5);
        assert!(dm.discard_actors.is_empty(), "random discard is not chosen by a player");
    }}
}

#[test]
fn discard_replacement_empty_hand_and_illegal_original_player_do_not_execute_both_arms() {
    for name in ["Bog Down", "Hypnotic Cloud", "Haunting Hymn", "Whispers of Emrakul"] { for definition in definitions(name) { for illegal in [false, true] {
        let mut game = game(); hand(&mut game, C, 4);
        if illegal { hand(&mut game, B, 4); }
        let mut paid = OptionalCostsPaid::default(); paid.mark_label_paid("Kicker"); paid.record_main_phase_cast(A);
        put_spell(&mut game, &definition, Target::Player(B), paid);
        if illegal { game.player_mut(B).unwrap().has_lost = true; }
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), if illegal { 4 } else { 0 });
        assert_eq!(game.player(C).unwrap().hand.len(), 4);
    }}}
}

#[test]
fn discard_choice_suspension_retries_the_selected_branch_once() {
    for definition in definitions("Hypnotic Cloud") {
        let mut game = game(); hand(&mut game, B, 5);
        let mut paid = OptionalCostsPaid::default(); paid.mark_label_paid("Kicker");
        put_spell(&mut game, &definition, Target::Player(B), paid);
        let mut dm = Choices { pause_discard: true, ..Default::default() };
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(game.stack.len(), 1); assert_eq!(game.player(B).unwrap().hand.len(), 5);
        dm.pause_discard = false; dm.pending = false; game = game.clone();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty()); assert_eq!(game.player(B).unwrap().hand.len(), 2);
    }
}

#[test]
fn conditional_discard_and_sacrifice_do_as_much_as_possible_for_a_legal_target() {
    for definition in definitions("Hypnotic Cloud") {
        let mut game = game(); hand(&mut game, B, 2);
        let mut paid = OptionalCostsPaid::default(); paid.mark_label_paid("Kicker");
        put_spell(&mut game, &definition, Target::Player(B), paid);
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert!(game.player(B).unwrap().hand.is_empty());
        assert_eq!(game.player(B).unwrap().graveyard.len(), 2);
    }
    for definition in definitions("Epicenter") { for threshold in [false, true] {
        let mut game = game(); let land = card(&mut game, A, Zone::Battlefield, vec![CardType::Land]);
        if threshold { graveyard_count(&mut game, A, 7); }
        put_spell(&mut game, &definition, Target::Player(B), OptionalCostsPaid::default());
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.object(land).is_none(), threshold, "a legal target with no lands does not prevent the all-player replacement");
    }}
}

#[test]
fn main_phase_evidence_is_actor_specific_and_missing_evidence_rolls_back_resolution() {
    for definition in definitions("Haunting Hymn") { for missing in [false, true] {
        let mut game = game(); hand(&mut game, C, 7);
        let mut paid = OptionalCostsPaid::default(); paid.record_main_phase_cast(A);
        let spell = put_spell(&mut game, &definition, Target::Player(C), paid);
        if missing {
            game.object_mut(spell).unwrap().optional_costs_paid.main_phase_caster = None;
            game.stack.last_mut().unwrap().optional_costs_paid.main_phase_caster = None;
            assert!(resolve_stack_entry_with(&mut game, &mut Choices::default()).is_err());
            assert_eq!(game.stack.len(), 1); assert_eq!(game.player(C).unwrap().hand.len(), 7);
        } else {
            game.set_current_controller(spell, B).unwrap();
            assert_eq!(game.current_controller(spell), Some(B));
            resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
            assert_eq!(game.player(C).unwrap().hand.len(), 5, "B did not cast this spell during B's main phase");
        }
    }}
}

#[test]
fn whispers_paid_cast_rechecks_delirium_after_spell_control_changes() {
    for definition in definitions("Whispers of Emrakul") { for new_controller_has_delirium in [false, true] {
        let mut game = game(); hand(&mut game, B, 5);
        let grave_owner = if new_controller_has_delirium { C } else { A };
        for kind in [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Instant] {
            card(&mut game, grave_owner, Zone::Graveyard, vec![kind]);
        }
        mana(&mut game, A, ManaSymbol::Black, 1); mana(&mut game, A, ManaSymbol::Colorless, 1);
        let mut dm = Choices { target: Some(Target::Player(B)), ..Default::default() };
        let spell = cast(&mut game, &definition, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        let declared = game.stack.last().unwrap().target_assignments.clone();
        game.set_current_controller(spell, C).unwrap();
        assert_eq!(game.current_controller(spell), Some(C));
        assert_eq!(game.stack.last().unwrap().targets, vec![Target::Player(B)]);
        assert_eq!(game.stack.last().unwrap().target_assignments, declared);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), if new_controller_has_delirium { 3 } else { 4 });
        assert!(dm.discard_actors.is_empty());
    }}
}

#[test]
fn recognized_discard_replacements_reject_complete_malformed_bodies_on_both_routes() {
    for tail in [
        "If this spell was kicked, that player discards three cards {B} instead.",
        "If this spell was kicked, that player discards three cards instead {B}.",
        "If this spell was kicked, that player discards three cards instead:.",
        "If this spell {B} was kicked, that player discards three cards instead.",
        "If this spell was kicked, that player discards three cards instead instead.",
        "If you cast this spell during your main phase {B}, that player discards four cards instead.",
    ] {
        let text = format!("Type: Sorcery\nTarget player discards a card. {tail}");
        assert!(compile_to_runtime_definition("Malformed local replacement", &text, false).is_err(), "{tail}");
        assert!(compile_to_artifact("Malformed local replacement", &text, false).is_err(), "{tail}");
    }
}

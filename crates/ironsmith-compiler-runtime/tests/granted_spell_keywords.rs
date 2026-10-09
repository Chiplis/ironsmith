//! Keywords granted to spells as they are cast (CR 601.2b, 601.2f): replicate
//! (CR 702.56), offspring (CR 702.175), conspire (CR 702.78) and demonstrate
//! (CR 702.144) granted by a typed `GrantSpellKeyword` static ability and
//! discovered uniformly while the matching spell is cast.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[path = "p02_line_families/compile.rs"]
mod compile;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const DJINN: &str = "Mana cost: {5}{U/R}{U/R}\nType: Creature — Djinn\nPower/Toughness: 3/5\n({U/R} can be paid with either {U} or {R}.)\nFlying\nEach instant and sorcery spell you cast has replicate. The replicate cost is equal to its mana cost. (When you cast it, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)";
const HATCHERY: &str = "Mana cost: {1}{G}\nType: Creature — Sliver\nPower/Toughness: 2/2\nReplicate {1}{G} (When you cast this spell, copy it for each time you paid its replicate cost.)\nEach Sliver spell you cast has replicate. The replicate cost is equal to its mana cost. (A copy of a permanent spell becomes a token.)";
const THREEFOLD: &str = "Mana cost: {3}\nType: Artifact\nWhen this artifact enters, scry 3.\nEach spell you cast that's exactly three colors has replicate {3}. (When you cast it, copy it for each time you paid its replicate cost. You may choose new targets for the copies. A copy of a permanent spell becomes a token.)";
const ZINNIA: &str = "Mana cost: {U}{R}{W}\nType: Legendary Creature — Bird Bard\nPower/Toughness: 1/3\nFlying\nZinnia gets +X/+0, where X is the number of other creatures you control with base power 1.\nCreature spells you cast gain offspring {2} as you cast them. (You may pay an additional {2} as you cast a creature spell. If you do, when that creature enters, create a 1/1 token copy of it.)";
const LECTURER: &str = "Mana cost: {4}{W}\nType: Creature — Kor Wizard\nPower/Toughness: 3/3\nCreature spells you cast have demonstrate. (Whenever you cast a creature spell, you may copy it. If you do, choose an opponent to also copy it. Each copy becomes a token.)";
const WORT: &str = "Mana cost: {4}{R/G}{R/G}\nType: Legendary Creature — Goblin Shaman\nPower/Toughness: 3/3\nWhen Wort enters, create two 1/1 red and green Goblin Warrior creature tokens.\nEach red or green instant or sorcery spell you cast has conspire. (As you cast the spell, you may tap two untapped creatures you control that share a color with it. When you do, copy it and you may choose new targets for the copy.)";

fn grant_debug(name: &str, text: &str) -> Vec<String> {
    compile::compile_both(name, text)
        .iter()
        .map(|definition| {
            let grants = compile::statics(definition, StaticAbilityId::GrantSpellKeyword);
            assert_eq!(grants.len(), 1, "{name}: one typed spell-keyword grant");
            format!("{:?}", grants[0])
        })
        .collect()
}

#[test]
fn granted_replicate_compiles_to_typed_spell_keyword_grants() {
    for (name, text, price) in [
        ("Djinn Illuminatus", DJINN, "SpellManaCost"),
        ("Hatchery Sliver", HATCHERY, "SpellManaCost"),
        ("Threefold Signal", THREEFOLD, "Fixed"),
    ] {
        for debug in grant_debug(name, text) {
            assert!(debug.contains("Replicate"), "{name}: {debug}");
            assert!(debug.contains(price), "{name}: {debug}");
        }
    }
    // Hatchery Sliver keeps its own printed replicate cost too.
    for definition in compile::compile_both("Hatchery Sliver", HATCHERY) {
        assert!(definition.optional_costs.iter().any(|cost| cost.kind
            == ironsmith::cost::OptionalCostKind::Replicate));
    }
    for debug in grant_debug("Djinn Illuminatus", DJINN) {
        assert!(debug.contains("Instant") && debug.contains("Sorcery"), "{debug}");
    }
    for debug in grant_debug("Hatchery Sliver", HATCHERY) {
        assert!(debug.contains("Sliver"), "{debug}");
    }
}

#[test]
fn granted_offspring_demonstrate_and_conspire_compile_to_typed_grants() {
    for debug in grant_debug("Zinnia, Valley's Voice", ZINNIA) {
        assert!(debug.contains("Offspring") && debug.contains("Fixed"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
    }
    for debug in grant_debug("Silverquill Lecturer", LECTURER) {
        assert!(debug.contains("Demonstrate") && debug.contains("Intrinsic"), "{debug}");
    }
    for debug in grant_debug("Wort, the Raidmother", WORT) {
        assert!(debug.contains("Conspire") && debug.contains("Intrinsic"), "{debug}");
        assert!(!debug.contains("KeywordMarker"), "conspire is no longer a display marker: {debug}");
    }
}

#[derive(Default)]
struct Choices {
    pay: Vec<&'static str>,
    seen_optional_costs: Vec<String>,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.seen_optional_costs
                .extend(ctx.options.iter().map(|option| option.description.clone()));
            return ctx
                .options
                .iter()
                .filter(|option| {
                    option.legal
                        && self.pay.iter().any(|needle| {
                            option.description.to_ascii_lowercase().contains(needle)
                        })
                })
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}

fn mana(game: &mut GameState, symbol: ManaSymbol, amount: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, amount);
}

fn fixture(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        .expect("the spell is castable");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack(game, &mut queue).unwrap();
            return game.stack.first().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast did not finish");
}

#[test]
fn threefold_signal_offers_replicate_only_to_three_color_spells_and_copies_per_payment() {
    for signal in compile::compile_both("Threefold Signal", THREEFOLD) {
        for three_colors in [true, false] {
            let mut game = game();
            game.create_object_from_definition(&signal, A, Zone::Battlefield);
            let text = if three_colors {
                "Mana cost: {W}{U}{B}\nType: Sorcery\nYou gain 1 life."
            } else {
                "Mana cost: {W}{U}\nType: Sorcery\nYou gain 1 life."
            };
            let spell = fixture("Replicate fixture", text);
            for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black] {
                mana(&mut game, symbol, 1);
            }
            mana(&mut game, ManaSymbol::Colorless, 3);
            let mut dm = Choices { pay: vec!["replicate"], ..Default::default() };
            cast(&mut game, &spell, &mut dm);
            let offered = dm.seen_optional_costs.iter().any(|label| label.to_ascii_lowercase().contains("replicate"));
            assert_eq!(offered, three_colors, "{:?}", dm.seen_optional_costs);
            // The spell plus one replicate trigger (CR 702.56b).
            assert_eq!(game.stack.len(), if three_colors { 2 } else { 1 });
            while !game.stack.is_empty() {
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(game.player(A).unwrap().life, if three_colors { 22 } else { 21 });
        }
    }
}

#[test]
fn djinn_illuminatus_replicate_costs_the_spells_own_mana_cost() {
    for djinn in compile::compile_both("Djinn Illuminatus", DJINN) {
        let mut game = game();
        game.create_object_from_definition(&djinn, A, Zone::Battlefield);
        let spell = fixture("Mana-cost replicate fixture", "Mana cost: {1}{R}\nType: Instant\nYou gain 1 life.");
        mana(&mut game, ManaSymbol::Red, 2);
        mana(&mut game, ManaSymbol::Colorless, 2);
        let mut dm = Choices { pay: vec!["replicate"], ..Default::default() };
        cast(&mut game, &spell, &mut dm);
        assert!(dm.seen_optional_costs.iter().any(|label| label.to_ascii_lowercase().contains("replicate")), "{:?}", dm.seen_optional_costs);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "paid {{1}}{{R}} twice");
        assert_eq!(game.stack.len(), 2);
    }
}

#[test]
fn wort_granted_conspire_remains_a_cast_time_tap_cost_with_its_copy_trigger() {
    for wort in compile::compile_both("Wort, the Raidmother", WORT) {
        for color_matches in [true, false] {
            let mut game = game();
            game.create_object_from_definition(&wort, A, Zone::Battlefield);
            let helper = fixture("Red helper", "Mana cost: {R}\nType: Creature — Goblin\nPower/Toughness: 1/1");
            let first = game.create_object_from_definition(&helper, A, Zone::Battlefield);
            let second = game.create_object_from_definition(&helper, A, Zone::Battlefield);
            let text = if color_matches {
                "Mana cost: {R}\nType: Instant\nYou gain 1 life."
            } else {
                "Mana cost: {U}\nType: Instant\nYou gain 1 life."
            };
            let spell = fixture("Conspire fixture", text);
            mana(&mut game, ManaSymbol::Red, 1);
            mana(&mut game, ManaSymbol::Blue, 1);
            let mut dm = Choices { pay: vec!["conspire"], ..Default::default() };
            cast(&mut game, &spell, &mut dm);
            let offered = dm.seen_optional_costs.iter().any(|label| label.to_ascii_lowercase().contains("conspire"));
            assert_eq!(offered, color_matches, "{:?}", dm.seen_optional_costs);
            for id in [first, second] {
                assert_eq!(game.is_tapped(id), color_matches);
            }
            assert_eq!(game.stack.len(), if color_matches { 2 } else { 1 });
            while !game.stack.is_empty() {
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            assert_eq!(game.player(A).unwrap().life, if color_matches { 22 } else { 21 });
        }
    }
}

#[test]
fn zinnia_offspring_paid_while_casting_creates_a_one_one_token_copy_on_entry() {
    for zinnia in compile::compile_both("Zinnia, Valley's Voice", ZINNIA) {
        for pay in [true, false] {
            let mut game = game();
            game.create_object_from_definition(&zinnia, A, Zone::Battlefield);
            let bear = fixture("Offspring bear", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 3/3");
            mana(&mut game, ManaSymbol::Green, 1);
            mana(&mut game, ManaSymbol::Colorless, 3);
            let mut dm = Choices { pay: if pay { vec!["offspring"] } else { vec![] }, ..Default::default() };
            cast(&mut game, &bear, &mut dm);
            assert!(dm.seen_optional_costs.iter().any(|label| label.to_ascii_lowercase().contains("offspring")));
            while !game.stack.is_empty() {
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            }
            let bears: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.name == "Offspring bear")
                .collect();
            assert_eq!(bears.len(), if pay { 2 } else { 1 });
            if pay {
                let token = bears.iter().find(|object| matches!(object.kind, ironsmith::object::ObjectKind::Token)).expect("token copy");
                assert_eq!(game.current_power(token.id), Some(1));
                assert_eq!(game.current_toughness(token.id), Some(1));
            }
        }
    }
}

#[test]
fn silverquill_lecturer_demonstrate_triggers_for_creature_spells_only() {
    for lecturer in compile::compile_both("Silverquill Lecturer", LECTURER) {
        for creature in [true, false] {
            let mut game = game();
            game.create_object_from_definition(&lecturer, A, Zone::Battlefield);
            let text = if creature {
                "Mana cost: {W}\nType: Creature — Kor\nPower/Toughness: 1/1"
            } else {
                "Mana cost: {W}\nType: Sorcery\nYou gain 1 life."
            };
            let spell = fixture("Demonstrate fixture", text);
            mana(&mut game, ManaSymbol::White, 1);
            let mut dm = Choices::default();
            cast(&mut game, &spell, &mut dm);
            assert_eq!(game.stack.len(), if creature { 2 } else { 1 });
            if creature {
                // Resolving demonstrate copies the spell for its caster and
                // for the chosen opponent (CR 702.144a).
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.stack.len(), 3);
                assert!(game.stack.iter().any(|entry| game.current_controller(entry.object_id) == Some(B)));
            }
        }
    }
}

/// Pays the first `count` legal offspring options.
struct PayOffspring {
    count: usize,
    offered: usize,
}

impl DecisionMaker for PayOffspring {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            let offspring: Vec<usize> = ctx
                .options
                .iter()
                .filter(|option| option.legal && option.description.to_ascii_lowercase().contains("offspring"))
                .map(|option| option.index)
                .collect();
            self.offered = offspring.len();
            return offspring.into_iter().take(self.count).collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
}

#[test]
fn printed_and_granted_offspring_each_trigger_only_for_their_own_payment() {
    for zinnia in compile::compile_both("Zinnia, Valley's Voice", ZINNIA) {
        for paid in [0usize, 1, 2] {
            let mut game = game();
            game.create_object_from_definition(&zinnia, A, Zone::Battlefield);
            let bear = fixture(
                "Printed offspring bear",
                "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 3/3\nOffspring {1}",
            );
            mana(&mut game, ManaSymbol::Green, 1);
            mana(&mut game, ManaSymbol::Colorless, 4);
            let spell = game.create_object_from_definition(&bear, A, Zone::Hand);
            let action = compute_legal_actions(&game, A)
                .unwrap()
                .into_iter()
                .find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
                .unwrap();
            let mut dm = PayOffspring { count: paid, offered: 0 };
            let mut queue = TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut progress = apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            while state.has_pending_action() {
                let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                progress =
                    apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
            }
            assert_eq!(dm.offered, 2, "printed and granted offspring are separate costs");
            while !game.stack.is_empty() {
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            }
            let tokens = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| {
                    object.name == "Printed offspring bear"
                        && matches!(object.kind, ironsmith::object::ObjectKind::Token)
                })
                .count();
            // CR 702.175b: one token per paid instance, never per payment seen.
            assert_eq!(tokens, paid);
        }
    }
}

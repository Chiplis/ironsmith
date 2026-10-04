//! Real compiled cards plus replacement-aware simultaneous discard/draw checks.
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::events::context::EventContext;
use ironsmith::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use ironsmith::game_state::StackEntry;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

#[derive(Debug, Clone)]
struct ObserveDrawPhase([usize; 2], Arc<AtomicUsize>);
impl ReplacementMatcher for ObserveDrawPhase {
    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &ironsmith::events::context::PreparedEventContext) -> bool {
        if event.event_kind() == EventKind::Draw {
            self.1.fetch_add(1, Ordering::Relaxed);
            for (player, expected) in [A, B].into_iter().zip(self.0) {
                assert_eq!(
                    ctx.game.player(player).unwrap().graveyard.len(),
                    expected,
                    "every player's discard must finish before the first draw"
                );
            }
        }
        false
    }
    fn display(&self) -> String {
        "Observe the discard/draw boundary".into()
    }
}

fn filler() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Sorcery])
        .build()
}

fn run_card(name: &str, text: &str, hands: [usize; 2], prevent_b: bool, draws: [usize; 2]) {
    let definition =
        ironsmith_registry::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let filler = filler();
    for (player, count) in [A, B].into_iter().zip(hands) {
        for _ in 0..count {
            game.create_object_from_definition(&filler, player, Zone::Hand);
        }
        for _ in 0..10 {
            game.create_object_from_definition(&filler, player, Zone::Library);
        }
    }
    let source = game.create_object_from_definition(&definition, A, Zone::Stack);
    let expected_discards = [hands[0], if prevent_b { 0 } else { hands[1] }];
    let observed_draws = Arc::new(AtomicUsize::new(0));
    game.effect_store
        .replacement_effects
        .add_effect(ReplacementEffect::with_matcher(
            source,
            A,
            ObserveDrawPhase(expected_discards, observed_draws.clone()),
            ReplacementAction::Prevent,
        ));
    if prevent_b {
        game.effect_store
            .replacement_effects
            .add_effect(ReplacementEffect::with_matcher(
                source,
                B,
                ironsmith::events::WouldDiscardMatcher::you(),
                ReplacementAction::Prevent,
            ));
    }
    game.push_to_stack(StackEntry::new(source, A));
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker)
        .unwrap_or_else(|error| {
            panic!(
                "{name} failed: {error:?}; program={:?}",
                definition.spell_effect
            )
        });
    for (index, player) in [A, B].into_iter().enumerate() {
        let retained = if prevent_b && player == B {
            hands[1]
        } else {
            0
        };
        assert_eq!(
            game.player(player).unwrap().hand.len(),
            draws[index] + retained,
            "{name}: draws for {player:?}"
        );
        assert_eq!(
            game.player(player).unwrap().library.len(),
            10 - draws[index]
        );
        assert_eq!(
            game.player(player).unwrap().graveyard.len(),
            expected_discards[index] + usize::from(player == A)
        );
    }
    assert!(game.stack.is_empty());
    if draws.iter().any(|count| *count > 0) {
        assert!(
            observed_draws.load(Ordering::Relaxed) > 0,
            "draw phase observer must run"
        );
    }
}

const WINDFALL: &str = "Each player discards their hand, then draws cards equal to the greatest number of cards a player discarded this way.";

#[test]
fn windfall_uses_collective_discard_count_before_any_draw() {
    run_card("Windfall", WINDFALL, [2, 5], false, [5, 5]);
}
#[test]
fn windfall_counts_successful_discards_after_replacements() {
    run_card("Windfall", WINDFALL, [2, 5], true, [2, 2]);
}
#[test]
fn windfall_empty_hands_draw_zero() {
    run_card("Windfall", WINDFALL, [0, 0], false, [0, 0]);
}
#[test]
fn wheel_of_fortune_discards_all_hands_before_seven_draws() {
    run_card(
        "Wheel of Fortune",
        "Each player discards their hand, then draws seven cards.",
        [2, 5],
        false,
        [7, 7],
    );
}

#[test]
fn simultaneous_hand_exchange_keeps_each_players_scalar_count() {
    run_card(
        "Hand exchange",
        "Each player discards their hand, then draws that many cards.",
        [2, 5],
        false,
        [2, 5],
    );
}

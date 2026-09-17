//! Measures the cost of the sliced priority-analysis loop against the
//! synchronous enumeration it replaces.
//!
//! The browser worker publishes a pass-only priority menu and then refines it
//! in bounded slices. Only mana-search node pops are budgeted; the rest of a
//! slice — the derived view, source discovery and candidate enumeration — is a
//! fixed cost paid again on every slice. These probes report that fixed cost so
//! a regression in it shows up as slice amplification rather than as a vague
//! "the browser feels slow".

use super::*;
use crate::ability::Ability;
use crate::cards::builders::CardDefinitionBuilder;
use crate::cost::TotalCost;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::CardType;
use crate::zone::Zone;
use std::time::Instant;

/// Mirrors `web/ui/src/lib/adaptive-work-budget.js` so the probe measures the
/// scheduling policy the browser actually runs, not an idealized one. A slice
/// costs `fixed + units * per_unit`; the two are estimated separately so a
/// dominant fixed cost grows the budget instead of collapsing it.
struct AdaptiveBudget {
    budget: usize,
    max: usize,
    target_ms: f64,
    samples: Vec<(usize, f64)>,
    fixed_ms: f64,
    per_unit_ms: f64,
    saturated: bool,
}

impl AdaptiveBudget {
    fn new(initial: usize, max: usize) -> Self {
        Self {
            budget: initial.max(1),
            max,
            target_ms: 4.0,
            samples: Vec::new(),
            fixed_ms: 0.0,
            per_unit_ms: 0.0,
            saturated: false,
        }
    }

    fn record(&mut self, units: usize, ms: f64) {
        match self.samples.iter_mut().find(|(u, _)| *u == units) {
            Some(entry) => entry.1 = entry.1.min(ms),
            None => self.samples.push((units, ms)),
        }
    }

    fn fit(&mut self) -> bool {
        if self.samples.len() < 2 {
            return false;
        }
        let low = self
            .samples
            .iter()
            .min_by_key(|(units, _)| *units)
            .copied()
            .unwrap();
        let high = self
            .samples
            .iter()
            .max_by_key(|(units, _)| *units)
            .copied()
            .unwrap();
        if high.0 <= low.0 {
            return false;
        }
        let slope = ((high.1 - low.1) / (high.0 - low.0) as f64).max(0.0);
        self.per_unit_ms = slope;
        self.fixed_ms = (low.1 - low.0 as f64 * slope).max(0.0).min(low.1);
        true
    }

    fn run<T>(&mut self, work: impl FnOnce(usize) -> (T, usize)) -> (T, f64) {
        let offered = self.budget;
        let started = Instant::now();
        let (result, spent) = work(offered);
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        let spent = spent.min(offered);
        if spent >= offered {
            self.record(offered, elapsed);
        } else {
            let overhead = (elapsed - spent as f64 * self.per_unit_ms).max(0.0);
            self.fixed_ms = if self.fixed_ms == 0.0 {
                overhead
            } else {
                self.fixed_ms.min(overhead)
            };
        }
        let next = if !self.fit() {
            if elapsed > self.target_ms * 4.0 {
                (offered / 2).max(1)
            } else {
                offered * 2
            }
        } else {
            let headroom = self.target_ms - self.fixed_ms;
            self.saturated = headroom <= 0.0;
            if self.saturated {
                offered * 4
            } else if self.per_unit_ms > 0.0 {
                (headroom / self.per_unit_ms).floor() as usize
            } else {
                offered * 2
            }
        };
        self.budget = next.clamp(1, self.max);
        (result, elapsed)
    }
}

pub(crate) struct ProbeBoard {
    pub game: GameState,
    pub player: PlayerId,
    pub sources: usize,
    pub hand: usize,
}

fn mana_permanent(name: &str, outputs: &[Vec<ManaSymbol>], snow: bool) -> CardDefinitionBuilder {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![
        if name.starts_with("Rock") {
            CardType::Artifact
        } else {
            CardType::Land
        },
    ]);
    if snow {
        builder = builder.supertypes(vec![crate::types::Supertype::Snow]);
    }
    for output in outputs {
        builder = builder.with_ability(Ability::mana(
            TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            output.clone(),
        ));
    }
    builder
}

/// A board shaped like a real midgame table: duals, a few any-color rocks, a
/// snow land, vanilla creatures for derived-view work, and a hand of spells
/// whose costs must each be solved.
pub(crate) fn probe_board(
    duals: usize,
    any_color: usize,
    hand_size: usize,
    creatures: usize,
) -> ProbeBoard {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId::from_index(0);
    let colors = [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ];
    let mut sources = 0;

    for index in 0..duals {
        let a = colors[index % colors.len()];
        let b = colors[(index / colors.len() + 1) % colors.len()];
        let definition = mana_permanent(
            &format!("Dual {index}"),
            &[vec![a], vec![b]],
            index % 7 == 0,
        )
        .build();
        game.create_object_from_definition(&definition, player, Zone::Battlefield);
        sources += 1;
    }
    for index in 0..any_color {
        let outputs: Vec<Vec<ManaSymbol>> = colors.iter().map(|color| vec![*color]).collect();
        let definition = mana_permanent(&format!("Rock {index}"), &outputs, false).build();
        game.create_object_from_definition(&definition, player, Zone::Battlefield);
        sources += 1;
    }
    for index in 0..creatures {
        let definition = CardDefinitionBuilder::new(CardId::new(), format!("Bear {index}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&definition, player, Zone::Battlefield);
    }
    for index in 0..hand_size {
        let generic = (index % 5) + 2;
        let pips = vec![
            vec![ManaSymbol::Generic(generic as u8)],
            vec![colors[index % colors.len()]],
            vec![colors[(index + 2) % colors.len()]],
        ];
        let definition = CardDefinitionBuilder::new(CardId::new(), format!("Spell {index}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(3, 3))
            .mana_cost(ManaCost::from_pips(pips))
            .build();
        game.create_object_from_definition(&definition, player, Zone::Hand);
    }
    // Sorcery-speed casts need the active player's main phase with an empty
    // stack; without it the hand never reaches the cost solver.
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();

    ProbeBoard {
        game,
        player,
        sources,
        hand: hand_size,
    }
}

pub(crate) struct SliceReport {
    pub one_shot_ms: f64,
    pub sliced_ms: f64,
    pub slices: usize,
    pub worst_slice_ms: f64,
    pub final_budget: usize,
}

impl SliceReport {
    pub fn amplification(&self) -> f64 {
        if self.one_shot_ms <= 0.0 {
            0.0
        } else {
            self.sliced_ms / self.one_shot_ms
        }
    }
}

/// Runs the enumeration once synchronously, then again through the sliced
/// session under the browser's budget policy, and reports both.
pub(crate) fn measure_slices(board: &ProbeBoard) -> SliceReport {
    let started = Instant::now();
    let baseline = crate::game_loop::analyze_priority_context(&board.game, board.player);
    let one_shot_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(baseline.analysis_complete);
    assert!(
        baseline.actions.len() > 1,
        "probe board must offer actions beyond passing, got {:?}",
        baseline.actions
    );

    let mut session = ManaAnalysisSession::default();
    let mut budget = AdaptiveBudget::new(8, 4096);
    let mut slices = 0usize;
    let mut worst_slice_ms = 0.0f64;
    let total = Instant::now();
    loop {
        let ((ctx, complete), elapsed) = budget.run(|units| {
            let outcome = session.run(units, || {
                crate::game_loop::analyze_priority_context(&board.game, board.player)
            });
            let spent = session.last_slice_nodes();
            (outcome, spent)
        });
        slices += 1;
        worst_slice_ms = worst_slice_ms.max(elapsed);
        if complete {
            assert_eq!(
                ctx.actions.len(),
                baseline.actions.len(),
                "sliced menu must match the synchronous menu"
            );
            break;
        }
        assert!(slices < 200_000, "analysis never settled");
    }

    SliceReport {
        one_shot_ms,
        sliced_ms: total.elapsed().as_secs_f64() * 1000.0,
        slices,
        worst_slice_ms,
        final_budget: budget.budget,
    }
}

/// Where the un-budgetable part of a slice goes. Everything here is re-paid on
/// every slice, so it sets the floor on slice wall time.
#[test]
#[ignore = "manual performance probe"]
fn priority_analysis_fixed_cost_breakdown() {
    let board = probe_board(60, 12, 30, 300);
    let _ = crate::game_loop::analyze_priority_context(&board.game, board.player);
    let perf = crate::decision::last_compute_legal_actions_perf().expect("perf recorded");
    let mut rows = vec![
        ("derived_view", perf.derived_view_ms),
        ("prewarm", perf.prewarm_ms),
        ("cast_context", perf.cast_context_ms),
        ("battlefield_ability_context", perf.battlefield_ability_context_ms),
        ("active_grant_zone_checks", perf.active_grant_zone_checks_ms),
        ("hand_summary", perf.hand_summary_ms),
        ("controlled_battlefield", perf.controlled_battlefield_ms),
        ("lands", perf.lands_ms),
        ("hand_casts", perf.hand_casts_ms),
        ("  can_cast_spell_with_view", perf.can_cast_spell_with_view_ms),
        ("  hand_casts_affordability", perf.hand_casts_affordability_ms),
        ("  hand_casts_cost_adjustment", perf.hand_casts_cost_adjustment_ms),
        ("  hand_casts_target_legality", perf.hand_casts_target_legality_ms),
        ("  compute_potential_mana_with_view", perf.compute_potential_mana_with_view_ms),
        ("hand_alternatives", perf.hand_alternatives_ms),
        ("battlefield_abilities", perf.battlefield_abilities_ms),
        ("  battlefield_ability_affordability", perf.battlefield_ability_affordability_ms),
        ("  battlefield_ability_precheck", perf.battlefield_ability_precheck_ms),
        ("non_battlefield_abilities", perf.non_battlefield_abilities_ms),
    ];
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("total {:.2} ms, {} actions", perf.total_ms, perf.action_count);
    for (label, ms) in rows {
        if ms >= 0.01 {
            println!("  {label:38} {ms:7.2} ms");
        }
    }
}

#[test]
#[ignore = "manual performance probe"]
fn priority_analysis_slice_cost_report() {
    println!(
        "{:>6} {:>10} {:>5} {:>6} {:>11} {:>11} {:>8} {:>13} {:>8} {:>7}",
        "duals",
        "any-color",
        "hand",
        "board",
        "one_shot_ms",
        "sliced_ms",
        "slices",
        "worst_slice_ms",
        "budget",
        "amp"
    );
    for (duals, any_color, hand, creatures) in [
        (10usize, 0usize, 4usize, 12usize),
        (20, 2, 7, 12),
        (30, 4, 7, 24),
        (40, 6, 10, 24),
        (40, 10, 20, 120),
        (60, 12, 30, 300),
    ] {
        let board = probe_board(duals, any_color, hand, creatures);
        let report = measure_slices(&board);
        println!(
            "{:>6} {:>10} {:>5} {:>6} {:>11.2} {:>11.2} {:>8} {:>13.2} {:>8} {:>6.1}x",
            duals,
            any_color,
            board.hand,
            creatures,
            report.one_shot_ms,
            report.sliced_ms,
            report.slices,
            report.worst_slice_ms,
            report.final_budget,
            report.amplification()
        );
        assert_eq!(board.sources, duals + any_color);
    }
}

/// One colour of mana on the battlefield, an ability that needs a different
/// colour, and the Cauldron's permission to spend mana as though it were any
/// colour for activated abilities of creatures you control.
fn mono_color_board(
    available: ManaSymbol,
    required: ManaSymbol,
    lands: usize,
    with_cauldron: bool,
) -> (GameState, PlayerId, ObjectId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId::from_index(0);
    for index in 0..lands {
        let definition =
            mana_permanent(&format!("Source {index}"), &[vec![available]], false).build();
        game.create_object_from_definition(&definition, player, Zone::Battlefield);
    }
    if with_cauldron {
        let mut creature_filter = crate::filter::ObjectFilter::default();
        creature_filter.card_types = vec![CardType::Creature];
        creature_filter.controller = Some(crate::target::PlayerFilter::You);
        let permission = ironsmith_core::ManaSpendPermission {
            player: crate::target::PlayerFilter::You,
            scope: ironsmith_core::ManaSpendScope::ActivationCostsOf(creature_filter),
            mode: ironsmith_core::value_model::ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        };
        let cauldron = CardDefinitionBuilder::new(CardId::new(), "Agatha's Soul Cauldron")
            .card_types(vec![CardType::Artifact])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::new(
                    crate::static_abilities::ManaSpendPermissionAbility::new(
                        permission,
                        "any color for creature activations".to_string(),
                    ),
                ),
            ))
            .build();
        game.create_object_from_definition(&cauldron, player, Zone::Battlefield);
    }
    let cost = crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::mana(
        ManaCost::from_pips(vec![vec![required], vec![required]]),
    )]);
    let adept = CardDefinitionBuilder::new(CardId::new(), "Cauldron Adept")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .with_ability(crate::ability::Ability::activated(
            cost,
            vec![crate::effect::Effect::draw(1)],
        ))
        .build();
    let adept_id = game.create_object_from_definition(&adept, player, Zone::Battlefield);
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();
    (game, player, adept_id)
}

/// Whether the menu offers the adept's activated ability at all. Note that an
/// activated ability with a mana cost is offered regardless of affordability by
/// design: mana is paid in the window opened after announcement, so the exact
/// check happens in the payment flow, which is what this probe measures.
fn ability_is_offered(game: &GameState, player: PlayerId, adept: ObjectId) -> bool {
    crate::decision::compute_legal_actions(game, player)
        .iter()
        .any(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. } if *source == adept
            )
        })
}

/// The Cauldron's permission makes every mana source able to pay every colored
/// pip. That is exactly the input that maximizes the payment planner's branching
/// factor, so this probe measures the planner (not the affordability solver) on
/// boards with and without the permission.
#[test]
#[ignore = "manual performance probe"]
fn agatha_payment_planner_report() {
    use crate::mana_payment::{
        check_mana_payment, plan_mana_payment, ManaPaymentRequest,
    };
    println!(
        "{:>8} {:>6} {:>5} {:>12} {:>12} {:>10} {:>9} {:>7}",
        "cauldron", "lands", "cost", "check_ms", "plan_ms", "plans", "nodes", "limited"
    );
    for lands in [8usize, 10, 12, 14, 16] {
        for with_cauldron in [false, true] {
            let (game, player, adept) =
                mono_color_board(ManaSymbol::Green, ManaSymbol::Blue, lands, with_cauldron);
            // {2}{U}{U}: generic pips are what force the planner to consider
            // every source, and the colored pips are what the permission
            // unlocks on an all-green board.
            let cost = ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
                vec![ManaSymbol::Blue],
            ]);
            let mut request = ManaPaymentRequest::new(
                player,
                adept,
                crate::costs::PaymentReason::ActivateAbility,
                cost,
            );
            request.allow_mana_abilities = true;

            let started = Instant::now();
            let check = check_mana_payment(&game, &request);
            let check_ms = started.elapsed().as_secs_f64() * 1000.0;

            let started = Instant::now();
            let plans = plan_mana_payment(&game, &request);
            let plan_ms = started.elapsed().as_secs_f64() * 1000.0;
            let perf = crate::mana_payment::last_mana_payment_perf();

            // The affordability solver answers the same yes/no question without
            // simulating activations on cloned states.
            let view = crate::derived_view::DerivedGameView::new(&game);
            let started = Instant::now();
            let affordable = super::can_pay_mana_cost_with_available_sources(
                &game,
                player,
                Some(adept),
                &request.cost,
                0,
                crate::costs::PaymentReason::ActivateAbility,
                &crate::player::ManaSpendPolicy::default(),
                false,
                &view,
            );
            let solver_ms = started.elapsed().as_secs_f64() * 1000.0;
            println!(
                "{:>8} {:>6} {:>5} {:>12.2} {:>12.2} {:>10} {:>9} {:>7} | solver {:>8.3} ms -> {}",
                with_cauldron,
                lands,
                "2UU",
                check_ms,
                plan_ms,
                plans.as_ref().map(Vec::len).unwrap_or(0),
                perf.visited_nodes,
                perf.search_limited,
                solver_ms,
                affordable
            );
            assert_eq!(
                affordable,
                check.is_ok(),
                "solver and planner must agree on payability (lands={lands}, cauldron={with_cauldron})"
            );
            // The menu offers the ability either way; affordability is settled
            // by the payment flow measured above, not by enumeration.
            assert!(ability_is_offered(&game, player, adept));
        }
    }
}

/// A board where no two mana sources are interchangeable: every land is a
/// distinct colour pair and every rock a distinct output set. Choice collapsing
/// cannot help here, so this is the honest worst case for the planner's search
/// and the board the remaining per-candidate costs must be measured against.
fn heterogeneous_board(
    pairs: usize,
    rocks: usize,
    with_cauldron: bool,
) -> (GameState, PlayerId, ObjectId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let player = PlayerId::from_index(0);
    // Deliberately no blue: the probe cost needs blue, so the board is payable
    // only through the Cauldron's permission. That is what forces the candidate
    // search instead of the analytic assignment.
    let colors = [
        ManaSymbol::White,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ];
    let mut made = 0usize;
    for first in 0..colors.len() {
        for second in (first + 1)..colors.len() {
            if made >= pairs {
                break;
            }
            let definition = mana_permanent(
                &format!("Dual {first}{second}"),
                &[vec![colors[first]], vec![colors[second]]],
                made % 3 == 0,
            )
            .build();
            game.create_object_from_definition(&definition, player, Zone::Battlefield);
            made += 1;
        }
    }
    for index in 0..rocks {
        let outputs: Vec<Vec<ManaSymbol>> = colors
            .iter()
            .cycle()
            .skip(index)
            .take(2 + index % 3)
            .map(|color| vec![*color])
            .collect();
        let definition = mana_permanent(&format!("Rock {index}"), &outputs, index % 2 == 0).build();
        game.create_object_from_definition(&definition, player, Zone::Battlefield);
    }
    if with_cauldron {
        let mut creature_filter = crate::filter::ObjectFilter::default();
        creature_filter.card_types = vec![CardType::Creature];
        creature_filter.controller = Some(crate::target::PlayerFilter::You);
        let permission = ironsmith_core::ManaSpendPermission {
            player: crate::target::PlayerFilter::You,
            scope: ironsmith_core::ManaSpendScope::ActivationCostsOf(creature_filter),
            mode: ironsmith_core::value_model::ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        };
        let cauldron = CardDefinitionBuilder::new(CardId::new(), "Agatha's Soul Cauldron")
            .card_types(vec![CardType::Artifact])
            .supertypes(vec![crate::types::Supertype::Legendary])
            .with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::new(
                    crate::static_abilities::ManaSpendPermissionAbility::new(
                        permission,
                        "any color for creature activations".to_string(),
                    ),
                ),
            ))
            .build();
        game.create_object_from_definition(&cauldron, player, Zone::Battlefield);
    }
    let adept = CardDefinitionBuilder::new(CardId::new(), "Adept")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let adept_id = game.create_object_from_definition(&adept, player, Zone::Battlefield);
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state();
    (game, player, adept_id)
}

#[test]
#[ignore = "manual performance probe"]
fn heterogeneous_payment_planner_report() {
    use crate::mana_payment::{plan_mana_payment, ManaPaymentRequest};
    println!(
        "{:>8} {:>6} {:>6} {:>12} {:>10} {:>9} {:>8}",
        "cauldron", "duals", "rocks", "plan_ms", "plans", "nodes", "limited"
    );
    for (pairs, rocks, with_cauldron) in [
        (4usize, 2usize, false),
        (6, 3, false),
        (10, 6, false),
        (4, 2, true),
        (6, 3, true),
        (8, 4, true),
        (10, 6, true),
    ] {
        let (game, player, adept) = heterogeneous_board(pairs, rocks, with_cauldron);
        let cost = ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]);
        let mut request = ManaPaymentRequest::new(
            player,
            adept,
            crate::costs::PaymentReason::ActivateAbility,
            cost,
        );
        request.allow_mana_abilities = true;
        let started = Instant::now();
        let plans = plan_mana_payment(&game, &request);
        let plan_ms = started.elapsed().as_secs_f64() * 1000.0;
        let perf = crate::mana_payment::last_mana_payment_perf();
        println!(
            "{:>8} {:>6} {:>6} {:>12.2} {:>10} {:>9} {:>8}",
            with_cauldron,
            pairs,
            rocks,
            plan_ms,
            plans.as_ref().map(Vec::len).unwrap_or(0),
            perf.visited_nodes,
            perf.search_limited
        );
        // Without the permission there is no blue at all, so only the
        // cauldron rows are expected to find a plan.
        assert_eq!(
            plans.is_ok(),
            with_cauldron,
            "payability should track the permission (duals={pairs}, rocks={rocks})"
        );
    }
}

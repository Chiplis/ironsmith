//! Screenshot-sized boards with compiled cards, earthbend, and triggered mana.
use ironsmith::cards::CardDefinition;
use ironsmith::costs::PaymentReason;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{
    EarthbendEffect, EffectContext as ExecutionContext, EffectExecutor, ResolvedTarget,
};
use ironsmith::filter::ObjectFilter;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm};
use ironsmith::game_state::Phase;
use ironsmith::ids::ObjectId;
use ironsmith::mana::ManaCost;
use ironsmith::mana_payment::{
    ManaPaymentRequest, execute_mana_payment_plan, mana_payment_activation_inventory,
    plan_first_mana_payment,
};
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ManaSymbol, PlayerId, Zone};
use ironsmith_tools::{
    compile_definition_from_payload, default_cards_path, load_card_payloads_by_names,
    normalize_lookup_name,
};
use std::{collections::BTreeMap, time::Instant};

fn emit(case: &str, op: &str, iteration: usize, start: Instant, detail: serde_json::Value) {
    println!(
        "{}",
        serde_json::json!({"case":case,"operation":op,"iteration":iteration,"ms":start.elapsed().as_secs_f64()*1000.,"detail":detail})
    );
}
fn add(
    game: &mut GameState,
    cards: &BTreeMap<String, CardDefinition>,
    name: &str,
    owner: PlayerId,
    zone: Zone,
) -> ObjectId {
    game.create_object_from_definition(&cards[name], owner, zone)
}
fn board(
    cards: &BTreeMap<String, CardDefinition>,
    cubs: usize,
    cauldron: bool,
    screenshot_taps: bool,
) -> (GameState, ObjectId, Option<ObjectId>, ObjectId) {
    let mut game = GameState::new(vec!["Alice P1".into(), "Alice P2".into()], 20);
    let p1 = PlayerId::from_index(0);
    let p2 = PlayerId::from_index(1);
    game.player_mut(p1).unwrap().life = 26;
    for name in [
        "Liliana the Faultless",
        "Hinterland Sanctifier",
        "Essence Channeler",
        "Haliya, Guided by Light",
        "Plains",
        "Plains",
        "Shattered Sanctum",
    ] {
        let id = add(&mut game, cards, name, p1, Zone::Battlefield);
        if name == "Essence Channeler" {
            game.add_counters(id, CounterType::PlusOnePlusOne, 4);
        }
    }
    add(&mut game, cards, "Icetill Explorer", p2, Zone::Battlefield);
    let mut forest = None;
    for n in 0..5 {
        let id = add(&mut game, cards, "Forest", p2, Zone::Battlefield);
        if n == 4 {
            forest = Some(id);
        } else if screenshot_taps {
            game.tap(id);
        }
    }
    let mut erode = None;
    for name in ["Erode", "Plains", "Shattered Sanctum", "Godless Shrine"] {
        let id = add(&mut game, cards, name, p1, Zone::Hand);
        if name == "Erode" {
            erode = Some(id);
        }
    }
    let spell = add(&mut game, cards, "Icetill Explorer", p2, Zone::Hand);
    let forest = forest.unwrap();
    let mut cub_id = None;
    for _ in 0..cubs {
        cub_id = Some(add(
            &mut game,
            cards,
            "Badgermole Cub",
            p2,
            Zone::Battlefield,
        ));
    }
    if let Some(cub) = cub_id {
        // Resolve Cub's earthbend instruction with this particular land chosen.
        // Everything else is a settled position, rather than replaying ETBs.
        let mut ctx = ExecutionContext::new_default(cub, p2)
            .with_targets(vec![ResolvedTarget::Object(forest)]);
        EarthbendEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control())),
            1,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let elves = add(&mut game, cards, "Llanowar Elves", p2, Zone::Battlefield);
        game.remove_summoning_sickness(elves);
        if cauldron {
            let source = add(
                &mut game,
                cards,
                "Agatha's Soul Cauldron",
                p2,
                Zone::Battlefield,
            );
            let exiled = add(&mut game, cards, "Llanowar Elves", p2, Zone::Exile);
            game.add_exiled_with_source_link(source, exiled);
            game.add_counters(elves, CounterType::PlusOnePlusOne, 1);
        }
    }
    game.turn.turn_number = 8;
    game.turn.active_player = p2;
    game.turn.priority_player = Some(p2);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.refresh_continuous_state().unwrap();
    game.take_pending_trigger_events();
    (game, spell, cub_id.map(|_| forest), erode.unwrap())
}
fn main() {
    let repeats = std::env::args()
        .nth(1)
        .unwrap_or("5".into())
        .parse::<usize>()
        .unwrap();
    let names = [
        "Liliana the Faultless",
        "Hinterland Sanctifier",
        "Essence Channeler",
        "Haliya, Guided by Light",
        "Plains",
        "Shattered Sanctum",
        "Godless Shrine",
        "Icetill Explorer",
        "Forest",
        "Erode",
        "Badgermole Cub",
        "Llanowar Elves",
        "Agatha's Soul Cauldron",
    ];
    let payloads = load_card_payloads_by_names(
        default_cards_path().to_str().unwrap(),
        &names.iter().map(|n| n.to_string()).collect::<Vec<_>>(),
    )
    .unwrap();
    let cards = names
        .into_iter()
        .map(|name| {
            (
                name.to_string(),
                compile_definition_from_payload(&payloads[&normalize_lookup_name(name)][0])
                    .unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    for (case, cubs, cauldron, taps) in [
        ("screenshot_sized", 0, false, false),
        ("one_cub_earthbend", 1, false, false),
        ("two_cubs_earthbend", 2, false, false),
        ("cub_cauldron", 1, true, false),
        ("cub_two_available_sources", 1, false, true),
    ] {
        let (game, spell, animated, erode) = board(&cards, cubs, cauldron, taps);
        let payer = PlayerId::from_index(1);
        let request = ManaPaymentRequest::new(
            payer,
            spell,
            PaymentReason::CastSpell,
            ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Green],
                vec![ManaSymbol::Green],
            ]),
        );
        let white_request = ManaPaymentRequest::new(
            PlayerId::from_index(0),
            erode,
            PaymentReason::CastSpell,
            ManaCost::from_pips(vec![vec![ManaSymbol::White]]),
        );
        for iteration in 0..repeats {
            let start = Instant::now();
            let white_options = mana_payment_activation_inventory(&game, &white_request);
            emit(
                case,
                "erode_inventory",
                iteration,
                start,
                serde_json::json!({"options":white_options.len()}),
            );
            let start = Instant::now();
            let white_plan = plan_first_mana_payment(&game, &white_request).unwrap();
            assert!(white_plan.payable);
            assert_eq!(white_plan.mana_ability_steps.len(), 1);
            emit(
                case,
                "erode_plan",
                iteration,
                start,
                serde_json::json!({"activations":white_plan.mana_ability_steps.len()}),
            );
            let start = Instant::now();
            let options = mana_payment_activation_inventory(&game, &request);
            emit(
                case,
                "inventory",
                iteration,
                start,
                serde_json::json!({"battlefield":game.battlefield.len(),"registered_replacements":game.effect_store.replacement_effects.effects().len(),"replacement_matchers":game.effect_store.replacement_effects.effects().iter().map(|effect|effect.matcher.as_ref().map(|matcher|matcher.display())).collect::<Vec<_>>(),"options":options.len(),"animated_output":animated.and_then(|id|options.iter().find(|o|o.source==id).map(|o|o.expected_mana.green))}),
            );
            let start = Instant::now();
            let plan = plan_first_mana_payment(&game, &request);
            let metrics = ironsmith::mana_payment::last_mana_payment_perf();
            emit(
                case,
                "plan",
                iteration,
                start,
                serde_json::json!({"payable":plan.is_ok(),"activations":plan.as_ref().ok().map(|p|p.mana_ability_steps.len()),"failure":plan.as_ref().err().map(|e|format!("{e:?}")),"analytic_selections":metrics.analytic_selections,"searched_selections":metrics.searched_selections,"visited_nodes":metrics.visited_nodes,"search_limited":metrics.search_limited}),
            );
            if taps {
                let plan = plan.as_ref().expect("Cub makes two available sources sufficient");
                assert_eq!(plan.mana_ability_steps.len(), 2);
            }
            if let Ok(plan) = plan {
                let mut staged = game.clone();
                let counters_before = staged.work_counters();
                let start = Instant::now();
                let result = execute_mana_payment_plan(
                    &mut staged,
                    &request,
                    &plan,
                    &mut SelectFirstDecisionMaker,
                );
                assert!(result.is_ok());
                emit(
                    case,
                    "commit",
                    iteration,
                    start,
                    serde_json::json!({"result":format!("{result:?}"),"green_remaining":staged.player(payer).unwrap().mana_pool.green,
                        "global_invalidations":staged.work_counters().continuous_global_invalidations.saturating_sub(counters_before.continuous_global_invalidations),
                        "static_regens":staged.work_counters().static_ability_regens.saturating_sub(counters_before.static_ability_regens),
                        "characteristics_recomputed":staged.work_counters().characteristics_full_recomputes.saturating_sub(counters_before.characteristics_full_recomputes),
                        "dependency_pairs":staged.work_counters().dependency_pairs_probed.saturating_sub(counters_before.dependency_pairs_probed)}),
                );
            }
            let start = Instant::now();
            let actions = compute_legal_actions(&game, payer).unwrap();
            emit(
                case,
                "legal_actions",
                iteration,
                start,
                serde_json::json!({"actions":actions.len()}),
            );
            if taps {
                assert!(actions.iter().any(|action| matches!(action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
                    "Cub-funded spell must appear in legal actions");
            }
            if let Some(source) = animated {
                let action=actions.iter().find(|a|matches!(a,LegalAction::ActivateManaAbility{source:id,..} if *id==source)).unwrap().clone();
                let mut staged = game.clone();
                let mut queue = TriggerQueue::default();
                let mut state = PriorityLoopState::new(game.players_in_game());
                let start = Instant::now();
                apply_priority_response_with_dm(
                    &mut staged,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::PriorityAction(action),
                    &mut SelectFirstDecisionMaker,
                )
                .unwrap();
                let actual = staged.player(payer).unwrap().mana_pool.green;
                assert_eq!(
                    options.iter().find(|option| option.source == source).unwrap().expected_mana.green,
                    actual,
                    "inventory and actual priority activation must agree",
                );
                assert_eq!(
                    actual,
                    1 + cubs as u32,
                    "each Cub must add green when the creature land is tapped"
                );
                emit(
                    case,
                    "priority_tap_animated_land",
                    iteration,
                    start,
                    serde_json::json!({"green":actual}),
                );
                if taps {
                    let action = actions
                        .iter()
                        .find(|action| {
                            matches!(action,
                        LegalAction::ActivateManaAbility { source, .. }
                        if game.object(*source).is_some_and(|o| o.name.as_ref()=="Llanowar Elves"))
                        })
                        .unwrap()
                        .clone();
                    let start = Instant::now();
                    apply_priority_response_with_dm(
                        &mut staged,
                        &mut queue,
                        &mut state,
                        &PriorityResponse::PriorityAction(action),
                        &mut SelectFirstDecisionMaker,
                    )
                    .unwrap();
                    let green = staged.player(payer).unwrap().mana_pool.green;
                    assert_eq!(green, 4);
                    let paid = staged.try_pay_mana_cost_with_reason(
                        payer,
                        Some(spell),
                        &request.cost,
                        0,
                        PaymentReason::CastSpell,
                    ).expect("checked fixture mana payment");
                    assert!(
                        paid,
                        "two creature sources plus Cub really can pay {{2}}{{G}}{{G}}"
                    );
                    emit(
                        case,
                        "actual_two_source_payment",
                        iteration,
                        start,
                        serde_json::json!({"green_before_payment":green,"paid":paid}),
                    );
                }
            }
            assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
        }
    }
}

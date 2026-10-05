//! UNVALIDATED source-authored regressions. No build/test execution requested.
//! Payment foundations for the seven frozen X-spending bodies, which remain
//! partial until their full compiler and effect programs are represented.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::mana::{
    ActualManaAllocation, ManaCost, ManaSpendingRestriction, ManaSymbol, XManaAllocation,
};
use ironsmith::mana_payment::{
    ManaPaymentFailure, ManaPaymentRequest, execute_mana_payment_plan, plan_first_mana_payment,
};
use ironsmith::{GameState, ObjectId, PlayerId};
use ironsmith_core::color::{Color, ColorSet};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn cost(x: u32, ordinary: u32, colors: ColorSet, cap: Option<u32>) -> ManaCost {
    let raw = ManaCost::from_symbols(vec![ManaSymbol::X])
        .add_generic(ordinary)
        .with_spending_restriction(ManaSpendingRestriction::OnX {
            colors,
            maximum_per_color: cap,
        })
        .bind_x_payment(x);
    // One rewrite expands X while preserving the unrestricted generic capacity.
    raw.with_pips(ManaCost::new().add_generic(x + ordinary).pips().to_vec())
}
fn request(payer: PlayerId, cost: ManaCost) -> ManaPaymentRequest {
    ManaPaymentRequest::new(
        payer,
        ObjectId::from_raw(900),
        ironsmith::costs::PaymentReason::CastSpell,
        cost,
    )
}
#[test]
fn fixed_black_base_taxes_and_as_though_permissions_do_not_pay_black_x() {
    let mut game = game();
    let mut priced = cost(2, 1, ColorSet::BLACK, None).add_generic(1);
    priced.push(ManaSymbol::Black);
    game.player_mut(A).unwrap().mana_pool.white = 5;
    let mut request = request(A, priced);
    request.spend_policy = ironsmith::player::ManaSpendPolicy::from_any_color(true);
    assert_eq!(
        plan_first_mana_payment(&game, &request),
        Err(ManaPaymentFailure::NoLegalPlan)
    );
    assert_eq!(game.player(A).unwrap().mana_pool.white, 5);
    game.player_mut(A).unwrap().mana_pool.white = 3;
    game.player_mut(A).unwrap().mana_pool.black = 2;
    let plan = plan_first_mana_payment(&game, &request).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.required_x_allocation(),
        Some(XManaAllocation([0, 0, 2, 0, 0]))
    );
    execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}
#[test]
fn reduction_allocation_is_selectable_hashed_and_paid_without_count_drift() {
    let mut game = game();
    game.player_mut(A).unwrap().mana_pool.black = 3;
    let mut priced = cost(3, 1, ColorSet::BLACK, None).reduce_generic(2);
    priced.push(ManaSymbol::Black);
    let mut one = request(A, priced);
    one.preferences.x_allocation = Some(XManaAllocation([0, 0, 1, 0, 0]));
    let mut two = one.clone();
    two.preferences.x_allocation = Some(XManaAllocation([0, 0, 2, 0, 0]));
    let first = plan_first_mana_payment(&game, &one).unwrap();
    let second = plan_first_mana_payment(&game, &two).unwrap();
    assert_ne!(first.request_hash, second.request_hash);
    assert_ne!(first.id, second.id);
    assert_eq!(
        execute_mana_payment_plan(&mut game, &two, &first, &mut SelectFirstDecisionMaker),
        Err(ManaPaymentFailure::StalePlan)
    );
    assert_eq!(game.player(A).unwrap().mana_pool.black, 3);
    assert_eq!(
        first.mana_cost_after_alternatives.required_x_allocation(),
        one.preferences.x_allocation
    );
    execute_mana_payment_plan(&mut game, &one, &first, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().mana_pool.black, 0);
}
#[test]
fn reduced_x_above_five_is_payable_with_five_actual_distinct_colors() {
    let mut game = game();
    for color in Color::ALL {
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::from_color(color), 1);
    }
    let request = request(
        A,
        cost(7, 0, Color::ALL.into_iter().collect(), Some(1)).reduce_generic(2),
    );
    let plan = plan_first_mana_payment(&game, &request).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.required_x_allocation(),
        Some(XManaAllocation([1; 5]))
    );
    execute_mana_payment_plan(&mut game, &request, &plan, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}
#[test]
fn assist_searches_actual_helper_allocations_instead_of_discounting_x() {
    let mut game = game();
    game.player_mut(A).unwrap().mana_pool.black = 2;
    game.player_mut(B).unwrap().mana_pool.colorless = 1;
    game.player_mut(B).unwrap().mana_pool.black = 1;
    let mut full = cost(2, 0, ColorSet::BLACK, None);
    full.push(ManaSymbol::Black);
    let remaining = full.reduce_generic(1);
    let completion = request(A, remaining.clone());
    let mut helper = request(B, ManaCost::new().add_generic(1));
    helper.assist_completion = Some(Box::new(completion));
    let plan = plan_first_mana_payment(&game, &helper).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.required_actual_payment(),
        Some(ActualManaAllocation([0, 0, 1, 0, 0, 0]))
    );
    execute_mana_payment_plan(&mut game, &helper, &plan, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(B).unwrap().mana_pool.colorless, 1);
    assert_eq!(game.player(B).unwrap().mana_pool.black, 0);
    let caster = request(A, remaining.with_prepaid_generic(vec![ManaSymbol::Black]));
    let plan = plan_first_mana_payment(&game, &caster).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.required_x_allocation(),
        Some(XManaAllocation([0, 0, 2, 0, 0]))
    );
    execute_mana_payment_plan(&mut game, &caster, &plan, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    // Colorless Assist cannot satisfy the same original X obligation.
    let mut negative = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    negative.player_mut(A).unwrap().mana_pool.black = 2;
    negative.player_mut(B).unwrap().mana_pool.colorless = 1;
    assert_eq!(
        plan_first_mana_payment(&negative, &helper),
        Err(ManaPaymentFailure::NoLegalPlan)
    );
    assert_eq!(negative.player(B).unwrap().mana_pool.colorless, 1);
}
#[test]
fn assist_and_caster_share_one_per_color_cap() {
    let mut game = game();
    game.player_mut(A).unwrap().mana_pool.black = 3;
    game.player_mut(B).unwrap().mana_pool.black = 1;
    game.player_mut(B).unwrap().mana_pool.red = 1;
    let remaining = cost(2, 2, Color::ALL.into_iter().collect(), Some(1)).reduce_generic(1);
    let mut helper = request(B, ManaCost::new().add_generic(1));
    helper.assist_completion = Some(Box::new(request(A, remaining)));
    let plan = plan_first_mana_payment(&game, &helper).unwrap();
    assert_eq!(
        plan.mana_cost_after_alternatives.required_actual_payment(),
        Some(ActualManaAllocation([0, 0, 0, 1, 0, 0]))
    );
    assert_eq!(
        game.player(A).unwrap().mana_pool.black,
        3,
        "preflight is speculative"
    );
    assert_eq!(game.player(B).unwrap().mana_pool.red, 1);
}

#[test]
fn committed_allocation_is_retained_separately_from_total_spent_and_announced_x() {
    let mut game = game();
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Allocation receipt",
        "Type: Sorcery\nDraw a card.",
        false,
    )
    .unwrap();
    let spell = game.create_object_from_definition(&definition, A, ironsmith::Zone::Stack);
    game.object_mut(spell).unwrap().x_value = Some(3);
    game.player_mut(A).unwrap().mana_pool.black = 3;
    let mut price = cost(3, 1, ColorSet::BLACK, None).reduce_generic(2);
    price.push(ManaSymbol::Black);
    let mut payment =
        ManaPaymentRequest::new(A, spell, ironsmith::costs::PaymentReason::CastSpell, price);
    payment.preferences.x_allocation = Some(XManaAllocation([0, 0, 1, 0, 0]));
    let plan = plan_first_mana_payment(&game, &payment).unwrap();
    execute_mana_payment_plan(&mut game, &payment, &plan, &mut SelectFirstDecisionMaker).unwrap();
    let object = game.object(spell).unwrap();
    assert_eq!(
        object.mana_spent_on_x,
        Some(XManaAllocation([0, 0, 1, 0, 0]))
    );
    assert_eq!(object.x_value, Some(3));
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(object, &game);
    assert_eq!(snapshot.mana_spent_on_x, object.mana_spent_on_x);
    let copy = ironsmith::object::Object::spell_copy_of(object, ObjectId::from_raw(901), A);
    assert_eq!(copy.mana_spent_on_x, Some(XManaAllocation::default()));
    let mut dm = SelectFirstDecisionMaker;
    let context = ironsmith::effects::EffectContext::new(spell, A, &mut dm);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &ironsmith::effect::Value::ManaSpentOnX(Color::Black),
            &context
        )
        .unwrap(),
        1
    );
    drop(context);
    game.object_mut(spell).unwrap().mana_spent_on_x = None;
    let context = ironsmith::effects::EffectContext::new(spell, A, &mut dm);
    assert!(matches!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &ironsmith::effect::Value::ManaSpentOnX(Color::Black),
            &context
        ),
        Err(ironsmith::effects::ExecutionError::UnresolvableValue(_))
    ));
    drop(context);
    game.object_mut(spell).unwrap().mana_spent_on_x = Some(XManaAllocation::default());
    let context = ironsmith::effects::EffectContext::new(spell, A, &mut dm);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &ironsmith::effect::Value::ManaSpentOnX(Color::Black),
            &context
        )
        .unwrap(),
        0
    );
    drop(context);
    game.object_mut(spell).unwrap().mana_spent_on_x =
        Some(XManaAllocation([0, 0, i32::MAX as u32 + 1, 0, 0]));
    let context = ironsmith::effects::EffectContext::new(spell, A, &mut dm);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value_wide(
            &game,
            &ironsmith::effect::Value::ManaSpentOnX(Color::Black),
            &context
        )
        .unwrap(),
        i64::from(i32::MAX) + 1,
        "wide amount evaluation retains the exact paid quantity",
    );
    drop(context);
    game.object_mut(spell).unwrap().mana_spent_on_x = snapshot.mana_spent_on_x;
    game.move_object_by_effect(spell, ironsmith::Zone::Graveyard);
    let context =
        ironsmith::effects::EffectContext::new(spell, A, &mut dm).with_source_snapshot(snapshot);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &ironsmith::effect::Value::ManaSpentOnX(Color::Black),
            &context
        )
        .unwrap(),
        1
    );
}

mod exact_cards {
    use super::*;
    use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{
        ManaPaymentContext, NumberContext, SelectOptionsContext, TargetsContext,
    };
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm, resolve_stack_entry_with,
    };
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::{GameProgress, Target, Zone};
    fn pair(name: &str) -> [ironsmith::cards::CardDefinition; 2] {
        let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../fixtures/consumer_mana_spending.json.fixture"
        ))
        .unwrap();
        let row = rows.iter().find(|row| row["name"] == name).unwrap();
        let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler_runtime::compile_to_artifact(
                name,
                row["text"].as_str().unwrap(),
                false,
            )
        });
        let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        artifact.validate().unwrap();
        let restored = ironsmith_compiled_artifact::CompiledCardArtifact::from_json(
            &artifact.to_json().unwrap(),
        )
        .unwrap();
        assert_eq!(restored, artifact);
        [
            direct,
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
                .unwrap(),
        ]
    }
    fn setup() -> GameState {
        let mut game = super::game();
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.active_player = A;
        game.turn.priority_player = Some(A);
        game.turn.step = None;
        game
    }
    fn printed(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
        text: &str,
    ) -> ObjectId {
        let definition =
            ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
        game.create_object_from_definition(&definition, owner, zone)
    }
    fn action(game: &GameState, object: ObjectId, activation: bool) -> LegalAction {
        compute_legal_actions(game, A)
            .unwrap()
            .into_iter()
            .find(|action| match action {
                LegalAction::ActivateAbility { source, .. } => activation && *source == object,
                LegalAction::CastSpell {
                    spell_id,
                    casting_method,
                    ..
                } => {
                    !activation
                        && *spell_id == object
                        && matches!(
                            casting_method,
                            ironsmith::alternative_cast::CastingMethod::Normal
                        )
                }
                _ => false,
            })
            .expect("a legal announcement")
    }
    #[derive(Default)]
    struct Choices {
        x: u32,
        target: Option<Target>,
        mode: usize,
        kicked: bool,
        cancel: bool,
        force_x: bool,
        allocation: Option<XManaAllocation>,
    }
    impl DecisionMaker for Choices {
        fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
            if ctx.is_x_value {
                if !self.force_x {
                    assert!(self.x <= ctx.max);
                }
                self.x
            } else {
                SelectFirstDecisionMaker.decide_number(game, ctx)
            }
        }
        fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            if let Some(target) = self.target {
                assert!(
                    ctx.requirements
                        .iter()
                        .any(|r| r.legal_targets.contains(&target))
                );
                vec![target]
            } else {
                SelectFirstDecisionMaker.decide_targets(game, ctx)
            }
        }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return if self.kicked {
                    vec![
                        ctx.options
                            .iter()
                            .find(|option| {
                                option.legal
                                    && option.description.to_ascii_lowercase().contains("kicker")
                            })
                            .unwrap()
                            .index,
                    ]
                } else {
                    vec![]
                };
            }
            if ctx.options.iter().any(|option| {
                option
                    .description
                    .to_ascii_lowercase()
                    .contains("prevent the next")
            }) {
                assert!(ctx.options[self.mode].legal);
                return vec![ctx.options[self.mode].index];
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            if let Some(allocation) = self.allocation
                && ctx.request.cost.has_x_spending_restriction()
                && ctx.request.preferences.x_allocation != Some(allocation)
            {
                let mut preferences = ctx.request.preferences.clone();
                preferences.x_allocation = Some(allocation);
                return ManaPaymentResponse::Replan { preferences };
            }
            if self.cancel {
                ManaPaymentResponse::Cancel
            } else {
                SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
            }
        }
    }
    fn announce(
        game: &mut GameState,
        action: LegalAction,
        choices: &mut Choices,
    ) -> Result<(), ironsmith::game_loop::GameLoopError> {
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut progress = apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            choices,
        )?;
        for _ in 0..40 {
            if state.pending_cast.is_none() && state.pending_activation.is_none() {
                return Ok(());
            }
            let GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("{progress:?}");
            };
            progress =
                apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices)?;
        }
        panic!("announcement failed to finish");
    }
    #[test]
    fn five_complete_x_spending_bodies_are_strict_and_round_trip() {
        for name in [
            "Atalya, Samite Master",
            "Consume Spirit",
            "Crimson Hellkite",
            "Crypt Rats",
            "Emblazoned Golem",
        ] {
            pair(name);
        }
    }
    #[test]
    fn different_modal_spending_rules_are_not_hoisted_into_one_global_constraint() {
        let text = "Type: Creature\nPower/Toughness: 1/1\n{X}, {T}: Choose one —\n• You gain X life. Spend only white mana on X.\n• This creature deals X damage to any target. Spend only red mana on X.";
        assert!(
            ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Different mode prices",
                text,
                false
            )
            .is_err()
        );
    }
    #[test]
    fn atalya_announces_and_pays_white_x_for_both_modes_and_prevents_only_that_amount() {
        for definition in pair("Atalya, Samite Master") {
            for mode in 0..2 {
                let mut game = setup();
                let atalya = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.remove_summoning_sickness(atalya);
                let target = printed(
                    &mut game,
                    B,
                    Zone::Battlefield,
                    "Large target",
                    "Type: Creature\nPower/Toughness: 6/6",
                );
                game.player_mut(A).unwrap().mana_pool.white = 3;
                let mut choices = Choices {
                    x: 3,
                    mode,
                    target: (mode == 0).then_some(Target::Object(target)),
                    ..Default::default()
                };
                let activation = action(&game, atalya, true);
                announce(&mut game, activation, &mut choices).unwrap();
                assert!(game.is_tapped(atalya));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                if mode == 1 {
                    assert_eq!(game.player(A).unwrap().life, 23);
                } else {
                    let bolt = printed(
                        &mut game,
                        A,
                        Zone::Hand,
                        "Five damage",
                        "Mana cost: {0}\nType: Instant\nThis spell deals 5 damage to target creature.",
                    );
                    let cast = action(&game, bolt, false);
                    announce(&mut game, cast, &mut choices).unwrap();
                    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                    assert_eq!(game.damage_on(target), 2);
                    assert_eq!(game.player(A).unwrap().life, 20);
                }
            }
        }
    }
    #[test]
    fn crimson_and_rats_keep_announced_x_distinct_from_tap_and_damage_recipients() {
        for name in ["Crimson Hellkite", "Crypt Rats"] {
            for definition in pair(name) {
                let mut game = setup();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.remove_summoning_sickness(source);
                let target = printed(
                    &mut game,
                    B,
                    Zone::Battlefield,
                    "Damage recipient",
                    "Type: Creature\nPower/Toughness: 6/6",
                );
                let hellkite = name == "Crimson Hellkite";
                game.player_mut(A).unwrap().mana_pool.add(
                    if hellkite {
                        ManaSymbol::Red
                    } else {
                        ManaSymbol::Black
                    },
                    2,
                );
                let mut choices = Choices {
                    x: 2,
                    target: hellkite.then_some(Target::Object(target)),
                    ..Default::default()
                };
                let activation = action(&game, source, true);
                announce(&mut game, activation, &mut choices).unwrap();
                assert_eq!(game.is_tapped(source), hellkite);
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.damage_on(target), 2);
                assert_eq!(game.player(A).unwrap().life, if hellkite { 20 } else { 18 });
                assert_eq!(game.player(B).unwrap().life, if hellkite { 20 } else { 18 });
                if !hellkite {
                    assert_eq!(game.damage_on(source), 2);
                    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
                    assert!(game.object(source).is_none());
                }
            }
        }
    }
    #[test]
    fn consume_spirit_gains_x_even_when_all_its_damage_is_prevented() {
        for definition in pair("Consume Spirit") {
            let mut game = setup();
            let target = printed(
                &mut game,
                B,
                Zone::Battlefield,
                "Protected creature",
                "Type: Creature\nPower/Toughness: 6/6\nPrevent all damage that would be dealt to this creature.",
            );
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            game.player_mut(A).unwrap().mana_pool.black = 4;
            game.player_mut(A).unwrap().mana_pool.red = 1;
            let mut choices = Choices {
                x: 3,
                target: Some(Target::Object(target)),
                ..Default::default()
            };
            let cast = action(&game, spell, false);
            announce(&mut game, cast, &mut choices).unwrap();
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.damage_on(target), 0);
            assert_eq!(game.player(A).unwrap().life, 23);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    }
    #[test]
    fn golem_kicker_can_announce_seven_after_reductions_and_declining_keeps_base_cost_plain() {
        for definition in pair("Emblazoned Golem") {
            for kicked in [false, true] {
                let mut game = setup();
                if kicked {
                    printed(
                        &mut game,
                        A,
                        Zone::Battlefield,
                        "Creature reduction",
                        "Type: Enchantment\nCreature spells you cast cost {4} less to cast.",
                    );
                    for color in Color::ALL {
                        game.player_mut(A)
                            .unwrap()
                            .mana_pool
                            .add(ManaSymbol::from_color(color), 1);
                    }
                } else {
                    game.player_mut(A).unwrap().mana_pool.colorless = 2;
                }
                let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = game.object(card).unwrap().stable_id;
                let mut choices = Choices {
                    x: if kicked { 7 } else { 0 },
                    kicked,
                    ..Default::default()
                };
                let cast = action(&game, card, false);
                announce(&mut game, cast, &mut choices).unwrap();
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                let golem = game
                    .battlefield
                    .iter()
                    .copied()
                    .find(|id| game.object(*id).unwrap().stable_id == stable)
                    .unwrap();
                assert_eq!(
                    game.object(golem)
                        .unwrap()
                        .counters
                        .get(&ironsmith::CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    if kicked { 7 } else { 0 }
                );
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
    #[test]
    fn cancel_x_payment_rolls_back_tap_and_mana_without_resolving_any_mode() {
        for definition in pair("Atalya, Samite Master") {
            let mut game = setup();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            game.player_mut(A).unwrap().mana_pool.white = 3;
            let activation = action(&game, source, true);
            let mut choices = Choices {
                x: 3,
                mode: 1,
                cancel: true,
                ..Default::default()
            };
            announce(&mut game, activation, &mut choices).unwrap();
            assert!(!game.is_tapped(source));
            assert_eq!(game.player(A).unwrap().mana_pool.white, 3);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert!(game.stack.is_empty());
        }
    }
    #[test]
    fn wrong_color_activation_after_announcing_x_does_not_become_an_ordinary_generic_price() {
        for name in ["Atalya, Samite Master", "Crimson Hellkite", "Crypt Rats"] {
            for definition in pair(name) {
                let mut game = setup();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                game.remove_summoning_sickness(source);
                let target = printed(
                    &mut game,
                    B,
                    Zone::Battlefield,
                    "Wrong-color recipient",
                    "Type: Creature\nPower/Toughness: 6/6",
                );
                let wrong = if name == "Crimson Hellkite" {
                    ManaSymbol::Black
                } else {
                    ManaSymbol::Red
                };
                game.player_mut(A).unwrap().mana_pool.add(wrong, 3);
                let mut choices = Choices {
                    x: 2,
                    // Deliberately submit the chosen value even if the menu
                    // already excludes it; the response must remain unpaid.
                    force_x: true,
                    mode: 1,
                    target: (name == "Crimson Hellkite").then_some(Target::Object(target)),
                    ..Default::default()
                };
                let activation = action(&game, source, true); // X=0 still permits announcement.
                assert!(announce(&mut game, activation, &mut choices).is_err());
                assert!(!game.is_tapped(source));
                assert_eq!(game.player(A).unwrap().mana_pool.amount(wrong), 3);
                assert_eq!(game.player(A).unwrap().life, 20);
                assert_eq!(game.damage_on(target), 0);
                assert!(game.stack.is_empty());
            }
        }
    }
    #[test]
    fn paid_x_receipt_stays_on_departure_history_but_a_later_fully_reduced_cast_is_zero() {
        let mut game = setup();
        let card = printed(
            &mut game,
            A,
            Zone::Hand,
            "Repeatable X receipt",
            "Mana cost: {X}\nType: Sorcery\nSpend only black mana on X.\nYou gain X life.",
        );
        let stable = game.object(card).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.black = 2;
        let mut choices = Choices {
            x: 2,
            ..Default::default()
        };
        let cast = action(&game, card, false);
        announce(&mut game, cast, &mut choices).unwrap();
        let paid = game.stack.last().unwrap().object_id;
        assert_eq!(
            game.object(paid).unwrap().mana_spent_on_x,
            Some(XManaAllocation([0, 0, 2, 0, 0]))
        );
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        let grave = game
            .player(A)
            .unwrap()
            .graveyard
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().stable_id == stable)
            .unwrap();
        assert_eq!(
            game.object(grave).unwrap().mana_spent_on_x,
            Some(XManaAllocation::default())
        );
        assert_eq!(
            game.turn_store
                .turn_history
                .source_last_known_snapshot(paid)
                .unwrap()
                .mana_spent_on_x,
            Some(XManaAllocation([0, 0, 2, 0, 0]))
        );
        let hand = game.move_object_by_effect(grave, Zone::Hand).unwrap();
        printed(
            &mut game,
            A,
            Zone::Battlefield,
            "Generic discount",
            "Type: Enchantment\nSorcery spells you cast cost {2} less to cast.",
        );
        let cast = action(&game, hand, false);
        announce(&mut game, cast, &mut choices).unwrap();
        let free = game.stack.last().unwrap().object_id;
        assert_ne!(paid, free);
        assert_eq!(game.object(free).unwrap().x_value, Some(2));
        assert_eq!(
            game.object(free).unwrap().mana_spent_on_x,
            Some(XManaAllocation::default())
        );
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(A).unwrap().life, 24);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
    #[test]
    fn both_capped_damage_bodies_compile_strictly_with_exact_metric_and_cast_receipt() {
        for name in ["Drain Life", "Soul Burn"] {
            for definition in pair(name) {
                let debug = format!("{definition:?}");
                assert!(debug.contains("DamageDealtCappedByRecipient"), "{debug}");
                assert_eq!(debug.contains("ManaSpentOnX"), name == "Soul Burn");
            }
        }
    }
    #[test]
    fn drain_life_uses_before_life_before_loyalty_and_current_toughness_caps() {
        for definition in pair("Drain Life") {
            for kind in 0..3 {
                let mut game = setup();
                let target = match kind {
                    0 => {
                        game.player_mut(B).unwrap().life = 2;
                        Target::Player(B)
                    }
                    1 => Target::Object(printed(
                        &mut game,
                        B,
                        Zone::Battlefield,
                        "Small creature",
                        "Type: Creature\nPower/Toughness: 2/2",
                    )),
                    _ => {
                        let planeswalker = printed(
                            &mut game,
                            B,
                            Zone::Battlefield,
                            "Low loyalty",
                            "Type: Planeswalker — Jace\nLoyalty: 0",
                        );
                        game.add_counters(planeswalker, ironsmith::CounterType::Loyalty, 2);
                        Target::Object(planeswalker)
                    }
                };
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                game.player_mut(A).unwrap().mana_pool.black = 6;
                game.player_mut(A).unwrap().mana_pool.red = 1;
                let mut choices = Choices {
                    x: 5,
                    target: Some(target),
                    ..Default::default()
                };
                let cast = action(&game, spell, false);
                announce(&mut game, cast, &mut choices).unwrap();
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.player(A).unwrap().life, 22);
                match target {
                    Target::Player(player) => assert_eq!(game.player(player).unwrap().life, -3),
                    Target::Object(object) if kind == 1 => assert_eq!(game.damage_on(object), 5),
                    Target::Object(object) => {
                        assert_eq!(game.object(object).unwrap().loyalty(), Some(0))
                    }
                }
            }
        }
    }
    #[test]
    fn soul_burn_counts_black_allocated_to_x_excluding_its_fixed_black_and_generic_base() {
        for definition in pair("Soul Burn") {
            let mut game = setup();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            game.player_mut(A).unwrap().mana_pool.black = 2;
            game.player_mut(A).unwrap().mana_pool.red = 4;
            let mut choices = Choices {
                x: 3,
                target: Some(Target::Player(B)),
                ..Default::default()
            };
            let cast = action(&game, spell, false);
            announce(&mut game, cast, &mut choices).unwrap();
            let stack = game.stack.last().unwrap().object_id;
            assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.black, 2);
            assert_eq!(
                game.object(stack)
                    .unwrap()
                    .mana_spent_on_x
                    .unwrap()
                    .of_color(Color::Black),
                1
            );
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.player(B).unwrap().life, 17);
            assert_eq!(game.player(A).unwrap().life, 21);
        }
    }
    #[test]
    fn soul_burn_reduction_allocation_is_a_real_locked_player_choice_with_distinct_gain() {
        for definition in pair("Soul Burn") {
            for black in [1, 3] {
                let mut game = setup();
                printed(
                    &mut game,
                    A,
                    Zone::Battlefield,
                    "Spell discount",
                    "Type: Enchantment\nSorcery spells you cast cost {2} less to cast.",
                );
                let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                game.player_mut(A).unwrap().mana_pool.black = 4;
                let mut choices = Choices {
                    x: 3,
                    target: Some(Target::Player(B)),
                    allocation: Some(XManaAllocation([0, 0, black, 0, 0])),
                    ..Default::default()
                };
                let cast = action(&game, spell, false);
                announce(&mut game, cast, &mut choices).unwrap();
                resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                assert_eq!(game.player(B).unwrap().life, 17);
                assert_eq!(game.player(A).unwrap().life, 20 + black as i32);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
    #[test]
    fn capped_drain_spells_gain_nothing_when_prevented_or_the_only_target_blinks() {
        for name in ["Drain Life", "Soul Burn"] {
            for definition in pair(name) {
                for blink in [false, true] {
                    let mut game = setup();
                    let target = printed(
                        &mut game,
                        B,
                        Zone::Battlefield,
                        "Protected target",
                        "Type: Creature\nPower/Toughness: 6/6\nPrevent all damage that would be dealt to this creature.",
                    );
                    let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
                    game.player_mut(A).unwrap().mana_pool.black = 4;
                    game.player_mut(A).unwrap().mana_pool.red =
                        if name == "Drain Life" { 1 } else { 2 };
                    let mut choices = Choices {
                        x: 3,
                        target: Some(Target::Object(target)),
                        ..Default::default()
                    };
                    let cast = action(&game, spell, false);
                    announce(&mut game, cast, &mut choices).unwrap();
                    let current = if blink {
                        let exile = game.move_object_by_effect(target, Zone::Exile).unwrap();
                        game.move_object_by_effect(exile, Zone::Battlefield)
                            .unwrap()
                    } else {
                        target
                    };
                    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
                    assert_eq!(game.damage_on(current), 0);
                    assert_eq!(game.player(A).unwrap().life, 20);
                }
            }
        }
    }
    #[test]
    fn damage_added_non_mana_toughness_overflow_is_typed_incomplete_and_rolls_back_resolution() {
        use ironsmith::effect::{Effect, Until};
        use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
        use ironsmith::target::{ChooseSpec, ObjectFilter};
        // Requires the checked final-characteristics numeric-range owner
        // integrated with the mana-retention scalar correction (a998d5dad).
        for definition in pair("Drain Life") {
            let mut game = setup();
            let target = printed(
                &mut game,
                B,
                Zone::Battlefield,
                "Overflow recipient",
                "Type: Creature\nPower/Toughness: 6/6",
            );
            let provider = printed(
                &mut game,
                B,
                Zone::Battlefield,
                "Overflow replacement",
                "Type: Artifact",
            );
            game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    provider,
                    B,
                    ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                        ObjectFilter::specific(target),
                    ),
                    ReplacementAction::Additionally(vec![Effect::pump(
                        0,
                        i32::MAX,
                        ChooseSpec::SpecificObject(target),
                        Until::EndOfTurn,
                    )]),
                ),
            );
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            game.player_mut(A).unwrap().mana_pool.black = 4;
            game.player_mut(A).unwrap().mana_pool.red = 1;
            let mut choices = Choices {
                x: 3,
                target: Some(Target::Object(target)),
                ..Default::default()
            };
            let cast = action(&game, spell, false);
            announce(&mut game, cast, &mut choices).unwrap();
            let stack = game.stack.last().unwrap().object_id;
            let error = resolve_stack_entry_with(&mut game, &mut choices).unwrap_err();
            assert!(
                matches!(
                    error,
                    ironsmith::game_loop::GameLoopError::ExecutionFailed(
                        ironsmith::effects::ExecutionError::ContinuousDiscovery(_)
                    )
                ),
                "{error:?}"
            );
            assert_eq!(game.stack.last().unwrap().object_id, stack);
            assert_eq!(game.damage_on(target), 0);
            assert_eq!(game.current_toughness(target), Some(6));
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                0,
                "the earlier cast payment remains committed"
            );
        }
    }
}

#[test]
fn capped_damage_receipt_preserves_original_target_current_wither_toughness_and_exact_lki() {
    use ironsmith::Zone;
    use ironsmith::effect::{
        Effect, EffectId, EffectMetric, EffectMetricSource, EffectOutcome, Value,
    };
    use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
    use ironsmith::replacement::{
        RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect,
    };
    use ironsmith::target::{ChooseSpec, ObjectFilter};
    fn metric(game: &GameState, source: ObjectId, outcome: EffectOutcome) -> i32 {
        let mut context = ExecutionContext::new_default(source, A);
        context.store_outcome(EffectId(1), outcome);
        ironsmith::effects::helpers::resolve_value(
            game,
            &Value::EffectMetric {
                effect_id: EffectId(1),
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::DamageDealtCappedByRecipient,
            },
            &context,
        )
        .unwrap()
    }
    let mut game = game();
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Damage source",
        "Type: Creature\nPower/Toughness: 6/6\nWither",
        false,
    )
    .unwrap();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let target_definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Five toughness",
        "Type: Creature\nPower/Toughness: 5/5",
        false,
    )
    .unwrap();
    let target = game.create_object_from_definition(&target_definition, B, Zone::Battlefield);
    let outcome = execute_effect(
        &mut game,
        &Effect::deal_damage(3, ChooseSpec::SpecificObject(target)),
        &mut ExecutionContext::new_default(source, A),
    )
    .unwrap();
    assert_eq!(game.current_toughness(target), Some(2));
    assert_eq!(metric(&game, source, outcome.clone()), 2);
    let exile = game.move_object_by_effect(target, Zone::Exile).unwrap();
    let successor = game
        .move_object_by_effect(exile, Zone::Battlefield)
        .unwrap();
    assert_eq!(game.current_toughness(successor), Some(5));
    assert_eq!(
        metric(&game, source, outcome),
        2,
        "the new incarnation cannot replace original-target LKI"
    );

    // A redirected event keeps the original creature's cap rather than the
    // recipient player's twenty life. The original damage amount is still 7.
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            A,
            ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                ObjectFilter::specific(successor),
            ),
            ReplacementAction::Redirect {
                target: RedirectTarget::ToPlayer(B),
                which: RedirectWhich::First,
            },
        ));
    let redirected = execute_effect(
        &mut game,
        &Effect::deal_damage(7, ChooseSpec::SpecificObject(successor)),
        &mut ExecutionContext::new_default(source, A),
    )
    .unwrap();
    assert_eq!(game.player(B).unwrap().life, 13);
    assert_eq!(metric(&game, source, redirected), 5);
}

#[test]
fn capped_damage_metric_ignores_auxiliary_replacement_added_damage() {
    use ironsmith::effect::{Effect, EffectId, EffectMetric, EffectMetricSource, Value};
    use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    let mut game = game();
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Receipt source",
        "Type: Artifact",
        false,
    )
    .unwrap();
    let source = game.create_object_from_definition(&definition, A, ironsmith::Zone::Battlefield);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            A,
            ironsmith::events::damage::matchers::DamageToPlayerMatcher::new(
                PlayerFilter::Specific(B),
            ),
            ReplacementAction::Additionally(vec![Effect::deal_damage(
                4,
                ChooseSpec::SpecificPlayer(A),
            )]),
        ));
    let outcome = execute_effect(
        &mut game,
        &Effect::deal_damage(3, ChooseSpec::SpecificPlayer(B)),
        &mut ExecutionContext::new_default(source, A),
    )
    .unwrap();
    assert_eq!(game.player(A).unwrap().life, 16);
    assert_eq!(game.player(B).unwrap().life, 17);
    let mut context = ExecutionContext::new_default(source, A);
    context.store_outcome(EffectId(1), outcome);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &Value::EffectMetric {
                effect_id: EffectId(1),
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::DamageDealtCappedByRecipient,
            },
            &context
        )
        .unwrap(),
        3
    );
}

#[test]
fn damage_added_type_removal_makes_live_and_departed_noncreature_toughness_zero() {
    use ironsmith::effect::{Effect, EffectId, EffectMetric, EffectMetricSource, Until, Value};
    use ironsmith::effects::{
        ApplyContinuousEffect, EffectContext as ExecutionContext, execute_effect,
    };
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::{ChooseSpec, ObjectFilter};
    for departed in [false, true] {
        let mut game = game();
        let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
            "Before creature",
            "Type: Creature\nPower/Toughness: 5/5",
            false,
        )
        .unwrap();
        let source =
            game.create_object_from_definition(&definition, A, ironsmith::Zone::Battlefield);
        let target =
            game.create_object_from_definition(&definition, B, ironsmith::Zone::Battlefield);
        let mut additions = vec![Effect::new(ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(target),
            ironsmith::continuous::Modification::SetCardTypes(vec![ironsmith::CardType::Artifact]),
            Until::EndOfTurn,
        ))];
        if departed {
            additions.push(Effect::exile(ChooseSpec::SpecificObject(target)));
        }
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                A,
                ironsmith::events::damage::matchers::DamageToObjectMatcher::new(
                    ObjectFilter::specific(target),
                ),
                ReplacementAction::Additionally(additions),
            ),
        );
        let outcome = execute_effect(
            &mut game,
            &Effect::deal_damage(3, ChooseSpec::SpecificObject(target)),
            &mut ExecutionContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(game.object(target).is_none(), departed);
        if !departed {
            assert!(!game.current_has_card_type(target, ironsmith::CardType::Creature));
        }
        let mut context = ExecutionContext::new_default(source, A);
        context.store_outcome(EffectId(1), outcome);
        assert_eq!(
            ironsmith::effects::helpers::resolve_value(
                &game,
                &Value::EffectMetric {
                    effect_id: EffectId(1),
                    source: EffectMetricSource::Outcome,
                    metric: EffectMetric::DamageDealtCappedByRecipient,
                },
                &context
            )
            .unwrap(),
            0
        );
    }
}

#[test]
fn capped_damage_does_not_invent_toughness_for_a_known_creature_with_missing_evidence() {
    use ironsmith::effect::{EffectId, EffectMetric, EffectMetricSource, EffectOutcome, Value};
    use ironsmith::effects::EffectContext;
    let mut game = game();
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Missing creature stats",
    )
    .card_types(vec![ironsmith::CardType::Creature])
    .build();
    let object = game.create_object_from_definition(&definition, B, ironsmith::Zone::Battlefield);
    let event = ironsmith::events::DamageEvent::with_cause(
        ObjectId::from_raw(999),
        ironsmith::events::DamageTarget::Object(object),
        2,
        false,
        ironsmith::events::cause::EventCause::effect(),
    );
    let outcome = EffectOutcome::count(2)
        .with_event(ironsmith::triggers::TriggerEvent::new_with_provenance(
            event,
            Default::default(),
        ))
        .with_execution_fact(ironsmith::effect::ExecutionFact::DamageRecipientBefore(
            ironsmith::effect::DamageRecipientBefore::Object {
                object,
                was_creature: true,
                loyalty: None,
            },
        ));
    let mut context = ironsmith::effects::EffectContext::new_default(ObjectId::from_raw(999), A);
    context.store_outcome(EffectId(1), outcome);
    assert!(matches!(
        ironsmith::effects::helpers::resolve_value(
            &game,
            &Value::EffectMetric {
                effect_id: EffectId(1),
                source: EffectMetricSource::Outcome,
                metric: EffectMetric::DamageDealtCappedByRecipient,
            },
            &context
        ),
        Err(ironsmith::effects::ExecutionError::UnresolvableValue(_))
    ));
}

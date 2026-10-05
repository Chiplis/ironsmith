//! UNVALIDATED implementation-first regressions for discard activation payments.
use ironsmith::Subtype;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::costs::{Cost, CostContext, PaymentReason};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    ManaPaymentContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::DiscardEffect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::target::PlayerFilter;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/discard_cost_values.json.fixture"
    ))
    .unwrap()
}
fn round_trip(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    round_trip(name, card["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let filler = CardBuilder::new(CardId::new(), "Library filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..10 {
        game.create_object_from_card(&filler, A, Zone::Library);
    }
    game
}
fn permanent(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    mv: u8,
    kind: CardType,
    zone: Zone,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
        .card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_card(&card, owner, zone)
}
fn payment_ability(def: &CardDefinition) -> usize {
    def.abilities
        .iter()
        .position(|ability| {
            matches!(&ability.kind,
        AbilityKind::Activated(a) if a.mana_cost.costs().iter().any(|cost|
            cost.effect_ref().is_some_and(|e| e.downcast_ref::<DiscardEffect>().is_some())))
        })
        .expect("printed discard activation")
}
fn action(def: &CardDefinition, source: ObjectId) -> LegalAction {
    LegalAction::ActivateAbility {
        source,
        ability_index: payment_ability(def),
    }
}
#[derive(Default)]
struct Choices {
    x: u32,
    max_x: Option<u32>,
    saw_x: bool,
    target: Option<Target>,
    forbidden: Vec<Target>,
    selections: std::collections::VecDeque<Vec<String>>,
    cancel: bool,
    cancelled: bool,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert_eq!(ctx.min, 0);
        if let Some(expected) = self.max_x {
            assert_eq!(ctx.max, expected);
        }
        assert!(self.x <= ctx.max);
        self.saw_x = true;
        self.x
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        for forbidden in &self.forbidden {
            assert!(
                !ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(forbidden))
            );
        }
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
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(names) = self.selections.pop_front() {
            names
                .iter()
                .map(|name| {
                    ctx.candidates
                        .iter()
                        .find(|candidate| {
                            candidate.legal
                                && game
                                    .object(candidate.id)
                                    .is_some_and(|object| &object.name == name)
                        })
                        .expect("chosen card is eligible")
                        .id
                })
                .collect()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, ctx)
        }
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if self.cancel {
            self.cancelled = true;
            ManaPaymentResponse::Cancel
        } else {
            SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
        }
    }
}
fn activate(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
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
    for _ in 0..40 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished activation: {progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn mana(game: &mut GameState, symbol: ManaSymbol, count: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, count);
}

fn hand_card(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    mv: u8,
    colors: ColorSet,
    historic: u8,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(mv)]]))
        .color_indicator(colors)
        .card_types(vec![if historic == 1 {
            CardType::Artifact
        } else if historic == 3 {
            CardType::Enchantment
        } else {
            CardType::Creature
        }])
        .supertypes(if historic == 2 {
            vec![ironsmith::Supertype::Legendary]
        } else {
            vec![]
        })
        .subtypes(if historic == 3 {
            vec![Subtype::Saga]
        } else {
            vec![]
        })
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&card, owner, Zone::Hand)
}
fn one_selection(name: &str) -> std::collections::VecDeque<Vec<String>> {
    [vec![name.into()]].into()
}
fn discard_cost(definition: &CardDefinition) -> Cost {
    let AbilityKind::Activated(ability) = &definition.abilities[payment_ability(definition)].kind
    else {
        unreachable!()
    };
    ability
        .mana_cost
        .costs()
        .iter()
        .find(|cost| {
            cost.effect_ref()
                .is_some_and(|effect| effect.downcast_ref::<DiscardEffect>().is_some())
        })
        .unwrap()
        .clone()
}

#[test]
fn exact_discard_cost_cards_have_no_unimplemented_content_after_transport() {
    let cards = fixtures();
    assert_eq!(cards.len(), 5);
    assert_eq!(
        cards
            .iter()
            .filter(|card| card["proposed_coverage"] == "source_implemented_all_validation_deferred")
            .count(),
        4
    );
    for card in cards
        .into_iter()
        .filter(|card| card["proposed_coverage"] == "source_implemented_all_validation_deferred")
    {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            payment_ability(&definition);
        }
    }
}

#[test]
fn knollspine_discards_an_exact_x_mana_value_card_and_deals_announced_x() {
    for definition in definitions("Knollspine Invocation") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = hand_card(&mut game, A, "Three-value payment", 3, ColorSet::BLUE, 0);
        let other = hand_card(&mut game, A, "Two-value card", 2, ColorSet::RED, 0);
        let foreign = hand_card(&mut game, B, "Foreign high value", 9, ColorSet::GREEN, 0);
        mana(&mut game, ManaSymbol::Colorless, 5);
        let mut dm = Choices {
            x: 3,
            max_x: Some(3),
            target: Some(Target::Player(B)),
            selections: one_selection("Three-value payment"),
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x);
        assert!(game.object(paid).is_none());
        assert_eq!(game.object(other).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        assert_eq!(game.player(B).unwrap().life, 20);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17);
    }
}

#[test]
fn knollspine_noncontiguous_x_values_and_mana_bounds_do_not_waive_discard() {
    for definition in definitions("Knollspine Invocation") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = hand_card(&mut game, A, "Three-value payment", 3, ColorSet::BLUE, 0);
        hand_card(&mut game, B, "Foreign two", 2, ColorSet::BLUE, 0);
        let cost = discard_cost(&definition);
        let mut dm = Choices::default();
        let mut ctx =
            CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
        assert!(
            cost.can_pay(&game, &ctx).is_ok(),
            "some X is payable before announcement"
        );
        ctx.x_value = Some(2);
        assert!(
            cost.can_pay(&game, &ctx).is_err(),
            "an X below the maximum can still have no matching card"
        );
        assert!(cost.pay(&mut game, &mut ctx).is_err());
        assert_eq!(game.object(paid).unwrap().zone, Zone::Hand);
        ctx.x_value = Some(3);
        assert!(cost.can_pay(&game, &ctx).is_ok());
        drop(ctx);
        hand_card(&mut game, A, "One-value payment", 1, ColorSet::GREEN, 0);
        mana(&mut game, ManaSymbol::Colorless, 2);
        let mut dm = Choices {
            x: 1,
            max_x: Some(2),
            target: Some(Target::Player(B)),
            selections: one_selection("One-value payment"),
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().life, 19);
    }
}

#[test]
fn sanctum_historic_discard_accepts_artifacts_legendaries_and_sagas_but_not_foreign_cards() {
    for definition in definitions("Sanctum Spirit") {
        for historic in [1, 2, 3] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let ordinary = hand_card(&mut game, A, "Ordinary", 2, ColorSet::GREEN, 0);
            let foreign = hand_card(&mut game, B, "Foreign artifact", 2, ColorSet::default(), 1);
            let act = action(&definition, source);
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&act));
            let selected = hand_card(&mut game, A, "Historic payer", 2, ColorSet::BLUE, historic);
            let mut dm = Choices {
                selections: one_selection("Historic payer"),
                ..Default::default()
            };
            activate(&mut game, act, &mut dm);
            assert!(game.object(selected).is_none());
            assert_eq!(game.object(ordinary).unwrap().zone, Zone::Hand);
            assert_eq!(game.object(foreign).unwrap().zone, Zone::Hand);
            assert!(!game.current_has_static_ability_id(
                source,
                ironsmith::static_abilities::StaticAbilityId::Indestructible
            ));
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.current_has_static_ability_id(
                source,
                ironsmith::static_abilities::StaticAbilityId::Indestructible
            ));
        }
    }
}

#[test]
fn krovikan_nonblack_cost_keeps_color_exclusion_and_black_branch_discards_only_a_drawn_card() {
    for definition in definitions("Krovikan Sorcerer") {
        for black_branch in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let black = hand_card(&mut game, A, "Black payer", 1, ColorSet::BLACK, 0);
            let nonblack = hand_card(&mut game, A, "Colorless payer", 1, ColorSet::default(), 0);
            let retained = hand_card(&mut game, A, "Unrelated old card", 1, ColorSet::GREEN, 0);
            let index = definition.abilities.iter().enumerate().filter_map(|(i, ability)| matches!(&ability.kind,
                AbilityKind::Activated(a) if a.mana_cost.costs().iter().any(|c| c.effect_ref().is_some_and(|e| e.downcast_ref::<DiscardEffect>().is_some()))).then_some(i)).nth(usize::from(black_branch)).unwrap();
            let mut dm = Choices {
                selections: one_selection(if black_branch {
                    "Black payer"
                } else {
                    "Colorless payer"
                }),
                ..Default::default()
            };
            activate(
                &mut game,
                LegalAction::ActivateAbility {
                    source,
                    ability_index: index,
                },
                &mut dm,
            );
            assert!(game.is_tapped(source));
            assert!(
                game.object(if black_branch { black } else { nonblack })
                    .is_none()
            );
            if black_branch {
                dm.selections.push_back(vec!["Library filler".into()]);
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.object(retained).unwrap().zone, Zone::Hand);
            assert_eq!(game.player(A).unwrap().hand.len(), 3);
            assert_eq!(
                game.player(A).unwrap().graveyard.len(),
                if black_branch { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn variable_count_discard_keeps_printed_gix_cost_and_announced_x_without_claiming_its_body() {
    // The printed cost is isolated because Gix's plural immediate permission is
    // an independent partial. This control still uses real announcement/payment.
    let text = "Type: Creature — Phyrexian Praetor\nPower/Toughness: 3/3\n{4}{B}{B}{B}, Discard X cards: Draw X cards.";
    for definition in round_trip("Printed X-discard cost control", text) {
        for x in [0, 2] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for i in 0..3 {
                hand_card(&mut game, A, &format!("Payment {i}"), 1, ColorSet::BLUE, 0);
            }
            mana(&mut game, ManaSymbol::Black, 7);
            let mut dm = Choices {
                x,
                max_x: Some(3),
                selections: [(0..x).map(|i| format!("Payment {i}")).collect()].into(),
                ..Default::default()
            };
            activate(&mut game, action(&definition, source), &mut dm);
            assert!(dm.saw_x);
            assert_eq!(game.player(A).unwrap().hand.len(), 3 - x as usize);
            assert_eq!(game.player(A).unwrap().graveyard.len(), x as usize);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            // X=0 does not prompt for an object choice; remove the unused plan.
            dm.selections.clear();
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(A).unwrap().hand.len(), 3);
        }
    }
}

#[test]
fn x_discard_rejects_overpayment_and_unbound_tags_without_mutation() {
    let mut game = game();
    let source = permanent(
        &mut game,
        A,
        "Source",
        1,
        CardType::Artifact,
        Zone::Battlefield,
    );
    let card = hand_card(&mut game, A, "Payment", 3, ColorSet::BLUE, 0);
    mana(&mut game, ManaSymbol::Blue, 4);
    let pool = game.player(A).unwrap().mana_pool.clone();
    let cost = Cost::effect(DiscardEffect::you(Value::X));
    let mut dm = Choices::default();
    let mut ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
    assert!(cost.can_pay(&game, &ctx).is_ok());
    assert!(cost.pay(&mut game, &mut ctx).is_err());
    ctx.x_value = Some(2);
    assert!(cost.can_pay(&game, &ctx).is_err());
    assert!(cost.pay(&mut game, &mut ctx).is_err());
    ctx.x_value = Some(1);
    let unknown = Cost::effect(DiscardEffect::new_with_filter(
        1,
        PlayerFilter::You,
        false,
        Some(ironsmith::target::ObjectFilter::tagged("unknown")),
    ));
    assert!(unknown.can_pay(&game, &ctx).is_err());
    assert_eq!(game.player(A).unwrap().hand, vec![card]);
    assert!(game.player(A).unwrap().graveyard.is_empty());
    assert_eq!(game.player(A).unwrap().mana_pool, pool);
    assert_eq!(game.player(A).unwrap().life, 20);
}

#[test]
fn kozilek_printed_cost_and_target_value_work_independently_of_draw_difference_body() {
    let text = "Type: Creature — Eldrazi\nPower/Toughness: 12/12\nDiscard a card with mana value X: Counter target spell with mana value X.";
    for definition in round_trip("Printed mana-value discard control", text) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        hand_card(&mut game, A, "Three-value payment", 3, ColorSet::BLUE, 0);
        let spell = compile_to_runtime_definition(
            "Countered spell",
            "Mana cost: {3}\nType: Instant\nYou gain 3 life.",
            false,
        )
        .unwrap();
        let hand = game.create_object_from_definition(&spell, B, Zone::Hand);
        let stable = game.object(hand).unwrap().stable_id;
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 3);
        game.turn.priority_player = Some(B);
        let cast = compute_legal_actions(&game, B).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = Choices::default();
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(cast),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("unfinished cast: {progress:?}")
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
        let target = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(target).unwrap().zone, Zone::Stack);
        game.turn.priority_player = Some(A);
        let mut dm = Choices {
            x: 3,
            max_x: Some(3),
            target: Some(Target::Object(target)),
            selections: one_selection("Three-value payment"),
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x);
        assert_eq!(game.stack.len(), 2);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack_is_empty());
        assert_eq!(game.player(B).unwrap().life, 20);
        assert!(
            game.player(B)
                .unwrap()
                .graveyard
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Countered spell")
        );
    }
}

#[test]
fn zero_x_mana_value_still_requires_and_discards_one_card() {
    for definition in definitions("Knollspine Invocation") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(
            !compute_legal_actions(&game, A)
                .unwrap()
                .contains(&action(&definition, source))
        );
        let card = hand_card(
            &mut game,
            A,
            "Zero-value payment",
            0,
            ColorSet::default(),
            0,
        );
        let mut dm = Choices {
            x: 0,
            max_x: Some(0),
            target: Some(Target::Player(B)),
            selections: one_selection("Zero-value payment"),
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x && game.object(card).is_none());
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(B).unwrap().life, 20);
    }
}

#[test]
fn cancelling_x_discard_restores_payment_and_does_not_expose_a_free_activation() {
    let text = "Type: Creature\nPower/Toughness: 3/3\n{4}{B}{B}{B}, Discard X cards: Draw X cards.";
    for definition in round_trip("X-discard cancellation control", text) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = hand_card(&mut game, A, "Payment", 3, ColorSet::BLUE, 0);
        mana(&mut game, ManaSymbol::Black, 7);
        let pool = game.player(A).unwrap().mana_pool.clone();
        let mut dm = Choices {
            x: 1,
            max_x: Some(1),
            selections: one_selection("Payment"),
            cancel: true,
            ..Default::default()
        };
        activate(&mut game, action(&definition, source), &mut dm);
        assert!(dm.saw_x && dm.cancelled);
        assert!(game.stack_is_empty());
        assert_eq!(game.player(A).unwrap().hand, vec![card]);
        assert!(game.player(A).unwrap().graveyard.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool, pool);
    }
}

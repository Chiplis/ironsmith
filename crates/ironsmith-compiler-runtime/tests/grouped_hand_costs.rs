//! UNVALIDATED implementation-first regressions for grouped hand activation costs.
use ironsmith::Subtype;
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::costs::{Cost, CostContext, PaymentReason};
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{ManaPaymentContext, SelectObjectsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{ChooseObjectsEffect, DiscardEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::object::CounterType;
use ironsmith::target::ObjectFilter;
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
        "../../../fixtures/grouped_hand_costs.json.fixture"
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
fn group_choice(effect: &Effect) -> Option<ChooseObjectsEffect> {
    if let Some(choose) = effect.downcast_ref::<ChooseObjectsEffect>() {
        if choose.filter.distinct_names || choose.filter.shares_name || choose.filter.shares_color {
            return Some(choose.clone());
        }
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = group_choice(child);
        }
    });
    found
}
fn payment_ability(def: &CardDefinition) -> usize {
    def.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(a) if a.mana_cost.costs().iter().filter_map(|cost| cost.effect_ref()).any(|effect| group_choice(effect).is_some())))
        .expect("printed grouped hand activation")
}
fn action(def: &CardDefinition, source: ObjectId) -> LegalAction {
    LegalAction::ActivateAbility {
        source,
        ability_index: payment_ability(def),
    }
}
#[derive(Default)]
struct Choices {
    objects: Vec<ObjectId>,
    expected_relation: Option<&'static str>,
    cancel: bool,
    cancelled: bool,
}
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(expected) = self.expected_relation {
            let filter = ctx
                .relation_filter
                .as_ref()
                .expect("group relation reaches the decision boundary");
            assert!(match expected {
                "color" => filter.shares_color,
                "name" => filter.shares_name,
                _ => filter.distinct_names,
            });
        }
        if self.objects.is_empty() {
            return SelectFirstDecisionMaker.decide_objects(game, ctx);
        }
        for id in &self.objects {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.legal && candidate.id == *id)
            );
        }
        self.objects.clone()
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

#[test]
fn exact_grouped_cost_cards_preserve_group_predicates_through_artifacts() {
    assert_eq!(fixtures().len(), 3);
    for name in [
        "Illuminated Folio",
        "Ormos, Archive Keeper",
        "Sphinx of the Chimes",
    ] {
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            payment_ability(&definition);
        }
    }
    let old = ObjectFilter::default();
    let mut json = serde_json::to_value(&old).unwrap();
    json.as_object_mut().unwrap().remove("shares_name");
    json.as_object_mut().unwrap().remove("shares_color");
    let restored: ObjectFilter = serde_json::from_value(json).unwrap();
    assert_eq!(
        restored, old,
        "old serialized filters default both new relations off"
    );
}

#[test]
fn folio_reveals_two_own_cards_with_a_common_color_and_does_not_discard_them() {
    for definition in definitions("Illuminated Folio") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let red = hand_card(&mut game, A, "Red", 1, ColorSet::RED, 0);
        let green = hand_card(&mut game, A, "Green", 1, ColorSet::GREEN, 0);
        let colorless = hand_card(&mut game, A, "Colorless", 1, ColorSet::default(), 0);
        hand_card(&mut game, B, "Foreign red", 1, ColorSet::RED, 0);
        mana(&mut game, ManaSymbol::Colorless, 1);
        let act = action(&definition, source);
        let pool = game.player(A).unwrap().mana_pool.clone();
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&act));
        assert_eq!(game.player(A).unwrap().hand, vec![red, green, colorless]);
        assert_eq!(game.player(A).unwrap().mana_pool, pool);
        let both = hand_card(
            &mut game,
            A,
            "Red green",
            2,
            ColorSet::RED.union(ColorSet::GREEN),
            0,
        );
        let mut dm = Choices {
            objects: vec![red, both],
            expected_relation: Some("color"),
            ..Default::default()
        };
        activate(&mut game, act, &mut dm);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(A).unwrap().hand.len(), 4);
        assert!(game.player(A).unwrap().graveyard.is_empty());
        let revealed: Vec<_> = game
            .turn_store
            .turn_history
            .event_records
            .iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter_map(|record| {
                record
                    .event
                    .downcast::<ironsmith::events::CardRevealedEvent>()
            })
            .collect();
        assert_eq!(revealed.len(), 2);
        dm.objects.clear();
        dm.expected_relation = None;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 5);
        assert_eq!(game.object(red).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(both).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn sphinx_requires_two_nonlands_sharing_a_name_in_the_payers_hand() {
    for definition in definitions("Sphinx of the Chimes") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let one = hand_card(&mut game, A, "Matching name", 1, ColorSet::BLUE, 0);
        let other = hand_card(&mut game, A, "Different", 1, ColorSet::GREEN, 0);
        let land = permanent(&mut game, A, "Matching name", 0, CardType::Land, Zone::Hand);
        let foreign = hand_card(&mut game, B, "Matching name", 1, ColorSet::BLUE, 0);
        let act = action(&definition, source);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&act));
        let two = hand_card(&mut game, A, "Matching name", 2, ColorSet::RED, 0);
        let mut dm = Choices {
            objects: vec![one, two],
            expected_relation: Some("name"),
            ..Default::default()
        };
        activate(&mut game, act, &mut dm);
        assert!(game.object(one).is_none() && game.object(two).is_none());
        for id in [other, land, foreign] {
            assert_eq!(game.object(id).unwrap().zone, Zone::Hand);
        }
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 2);
        dm.objects.clear();
        dm.expected_relation = None;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 6);
    }
}

#[test]
fn ormos_discards_three_different_names_and_replaces_only_its_controllers_empty_draws() {
    for definition in definitions("Ormos, Archive Keeper") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.set_current_controller(source, A).unwrap();
        let duplicate = hand_card(&mut game, A, "Alpha", 1, ColorSet::BLUE, 0);
        let alpha = hand_card(&mut game, A, "Alpha", 1, ColorSet::GREEN, 0);
        let beta = hand_card(&mut game, A, "Beta", 1, ColorSet::RED, 0);
        mana(&mut game, ManaSymbol::Blue, 3);
        let act = action(&definition, source);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&act));
        let gamma = hand_card(&mut game, A, "Gamma", 1, ColorSet::WHITE, 0);
        let to_remove: Vec<_> = game
            .player(A)
            .unwrap()
            .library
            .iter()
            .copied()
            .skip(3)
            .collect();
        for id in to_remove {
            game.move_object_by_effect(id, Zone::Exile).unwrap();
        }
        assert_eq!(game.player(A).unwrap().library.len(), 3);
        assert!(game.player(B).unwrap().library.is_empty());
        let mut dm = Choices {
            objects: vec![alpha, beta, gamma],
            expected_relation: Some("different"),
            ..Default::default()
        };
        activate(&mut game, act, &mut dm);
        assert_eq!(game.player(A).unwrap().hand, vec![duplicate]);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        dm.objects.clear();
        dm.expected_relation = None;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            4,
            "three cards drawn before its controller's library becomes empty"
        );
        assert_eq!(
            game.object(source)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne),
            Some(&10),
            "two remaining draws each put five counters on the actual source"
        );
        assert!(game.player(A).unwrap().library.is_empty());
    }
}

#[test]
fn group_defaults_find_a_valid_pair_and_cancellation_restores_the_ability_cost() {
    for definition in definitions("Illuminated Folio") {
        for cancel in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            hand_card(&mut game, A, "Blue", 1, ColorSet::BLUE, 0);
            hand_card(&mut game, A, "Green one", 1, ColorSet::GREEN, 0);
            hand_card(&mut game, A, "Green two", 1, ColorSet::GREEN, 0);
            mana(&mut game, ManaSymbol::Colorless, 1);
            let hand = game.player(A).unwrap().hand.clone();
            let mut dm = Choices {
                cancel,
                expected_relation: Some("color"),
                ..Default::default()
            };
            activate(&mut game, action(&definition, source), &mut dm);
            assert_eq!(game.player(A).unwrap().hand, hand);
            assert_eq!(game.is_tapped(source), !cancel);
            assert_eq!(game.stack_is_empty(), cancel);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), u32::from(cancel));
            if cancel {
                assert!(dm.cancelled);
            }
        }
    }
}

#[test]
fn whole_group_is_an_intersection_and_stale_reveal_or_discard_tags_fail_closed() {
    use ironsmith::effects::{EffectContext, EffectExecutor, execute_effect};
    let mut game = game();
    let source = permanent(
        &mut game,
        A,
        "Source",
        1,
        CardType::Artifact,
        Zone::Battlefield,
    );
    let rg = hand_card(
        &mut game,
        A,
        "RG",
        1,
        ColorSet::RED.union(ColorSet::GREEN),
        0,
    );
    let gu = hand_card(
        &mut game,
        A,
        "GU",
        1,
        ColorSet::GREEN.union(ColorSet::BLUE),
        0,
    );
    let ur = hand_card(
        &mut game,
        A,
        "UR",
        1,
        ColorSet::BLUE.union(ColorSet::RED),
        0,
    );
    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        shares_color: true,
        ..Default::default()
    };
    let choose = ChooseObjectsEffect::new(filter, 3, PlayerFilter::You, "group");
    assert!(
        choose.can_execute_as_cost(&game, source, A).is_err(),
        "pairwise color sharing is insufficient without a common color"
    );
    let mut dm = Choices {
        objects: vec![rg, gu, ur],
        expected_relation: Some("color"),
        ..Default::default()
    };
    let mut ctx = EffectContext::new_default(source, A).with_decision_maker(&mut dm);
    assert!(execute_effect(&mut game, &Effect::new(choose), &mut ctx).is_err());
    drop(ctx);
    assert_eq!(game.player(A).unwrap().hand, vec![rg, gu, ur]);
    let snapshot =
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(rg).unwrap(), &game);
    let grave = game.move_object_by_effect(rg, Zone::Graveyard).unwrap();
    let returned = game.move_object_by_effect(grave, Zone::Hand).unwrap();
    assert_ne!(returned, rg);
    let mut ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
    ctx.tagged_objects.insert("stale".into(), vec![snapshot]);
    let reveal = Cost::try_effect(Effect::new(ironsmith::effects::RevealTaggedEffect::new(
        "stale",
    )))
    .unwrap();
    let discard = Cost::try_effect(Effect::new(DiscardEffect::new_with_filter(
        1,
        PlayerFilter::You,
        false,
        Some(ObjectFilter::tagged("stale").in_zone(Zone::Hand)),
    )))
    .unwrap();
    assert!(reveal.can_pay(&game, &ctx).is_err());
    assert!(discard.can_pay(&game, &ctx).is_err());
    assert_eq!(game.object(returned).unwrap().zone, Zone::Hand);
}

#[test]
fn existing_conditional_draw_bodies_keep_their_distinct_instead_boundaries() {
    // The Phial grammar guard is owned by af8574ba. Keep both permitted marker
    // positions when this branch is integrated beside that correction.
    for body in [
        "If you would draw a card while you have no cards in hand, instead draw two cards.",
        "If you would draw a card while you have no cards in hand, draw two cards instead.",
        "If you would draw a card while your library has no cards in it, you win the game instead.",
    ] {
        for definition in round_trip(
            "Conditional draw compatibility control",
            &format!("Type: Enchantment\n{body}"),
        ) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        }
    }
    for body in [
        "If you would draw a card while you have no cards in hand, draw two cards.",
        "If you would draw a card while you have no cards in hand, instead draw two cards instead.",
        "If you would draw a card while your library has no cards in it, instead put five +1/+1 counters on this enchantment instead.",
    ] {
        assert!(
            compile_to_runtime_definition(
                "Malformed replacement control",
                &format!("Type: Enchantment\n{body}"),
                false
            )
            .is_err(),
            "accepted malformed body: {body}"
        );
    }
}

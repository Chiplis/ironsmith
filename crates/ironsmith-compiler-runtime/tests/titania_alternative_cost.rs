//! UNVALIDATED source-stage regressions. No compiler/test execution is claimed.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../fixtures/titania_alternative_cost.json.fixture")).unwrap()
}

fn definitions() -> [CardDefinition; 2] {
    let fixture = fixture();
    let card = &fixture["card"];
    assert_eq!(card["oracle_id"], "e380e37d-926b-4a4b-a275-7844bf4956d5");
    let name = card["name"].as_str().unwrap();
    let text = fixture["text"].as_str().unwrap();
    let (result, losses) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.expect("the complete official body compiles without a panic");
    assert!(!losses.is_lossy(), "{}", losses.reasons_text());
    let (result, losses) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.expect("the complete official body has a typed artifact");
    assert!(!losses.is_lossy(), "{}", losses.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    assert_eq!(direct.canonical_text, restored.canonical_text);
    [direct, restored]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Caster".into(), "Ward controller".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn hand_card(game: &mut GameState, player: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Discard resource")
        .card_types(vec![CardType::Artifact]).build();
    game.create_object_from_card(&card, player, Zone::Hand)
}

fn mana(game: &mut GameState, player: PlayerId, symbol: ManaSymbol, amount: u32) {
    game.player_mut(player).unwrap().mana_pool.add(symbol, amount);
}

struct Choices {
    discard: bool,
    hybrid: ManaSymbol,
    card: Option<ObjectId>,
    target: Option<ObjectId>,
    pay_ward: bool,
    cancel_mana: bool,
    cancelled: bool,
    invalid_multiple: bool,
    branch_prompts: usize,
    ward_prompts: usize,
}

impl Choices {
    fn new(discard: bool, card: Option<ObjectId>) -> Self {
        Self { discard, hybrid: ManaSymbol::Black, card, target: None, pay_ward: true, cancel_mana: false,
            cancelled: false, invalid_multiple: false, branch_prompts: 0, ward_prompts: 0 }
    }
}

impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.options.iter().any(|option| option.description.to_ascii_lowercase().contains("discard")) {
            assert_eq!(context.player, A, "only the cast/ward payer announces a branch");
            self.branch_prompts += 1;
            if self.invalid_multiple { return vec![0, 1]; }
            let option = context.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains("discard") == self.discard
            }).expect("the requested branch is offered");
            vec![option.index]
        } else {
            SelectFirstDecisionMaker.decide_options(game, context)
        }
    }
    fn decide_objects(&mut self, _game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(context.player, A, "discard comes from the payer's hand");
        let card = self.card.expect("this case has a discard resource");
        assert!(context.candidates.iter().any(|candidate| candidate.id == card && candidate.legal));
        vec![card]
    }
    fn decide_targets(&mut self, _game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.target.unwrap());
        assert!(context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&target)));
        vec![target]
    }
    fn decide_boolean(&mut self, _game: &GameState, context: &BooleanContext) -> bool {
        assert_eq!(context.player, A);
        self.ward_prompts += 1;
        self.pay_ward
    }
    fn decide_mana_payment(&mut self, game: &GameState, context: &ManaPaymentContext) -> ManaPaymentResponse {
        if self.cancel_mana {
            self.cancelled = true;
            ManaPaymentResponse::Cancel
        } else {
            SelectFirstDecisionMaker.decide_mana_payment(game, context)
        }
    }
}

fn cast(game: &mut GameState, spell: ObjectId, choices: &mut Choices, queue: &mut TriggerQueue)
    -> Result<(), ironsmith::game_loop::GameLoopError>
{
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand, casting_method: CastingMethod::Normal,
        }), choices)?;
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { return Ok(()); }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("unfinished cast: {progress:?}");
        };
        progress = if let ironsmith::decisions::context::DecisionContext::HybridChoice(hybrid) = &context {
            assert_eq!(hybrid.player, A);
            let selected = hybrid.options.iter().find(|option| option.symbol == choices.hybrid)
                .expect("the selected funding color is one of the printed hybrid alternatives");
            apply_priority_response_with_dm(game, queue, &mut state,
                &PriorityResponse::HybridChoice(selected.index), choices)?
        } else {
            apply_decision_context_with_dm(game, queue, &mut state, &context, choices)?
        };
    }
    panic!("cast did not reach a terminal state");
}

#[test]
fn actual_metadata_whole_body_retains_two_distinct_payment_owners() {
    for definition in definitions() {
        assert_eq!(definition.card.name, "Titania, Rugged Rumbler");
        assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), "{2}{B/G}");
        assert_eq!(definition.card.card_types, vec![CardType::Creature]);
        assert!(definition.card.supertypes.contains(&Supertype::Legendary));
        assert!(definition.card.subtypes.contains(&Subtype::Human));
        assert!(definition.card.subtypes.contains(&Subtype::Villain));
        assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(5, 5)));
        let [cost] = definition.additional_cost.as_all().unwrap() else { panic!("one modal additional price"); };
        let modal = cost.effect_ref().unwrap().downcast_ref::<ironsmith::effects::ChooseModeEffect>().unwrap();
        assert_eq!(modal.modes.len(), 2);
        let ward = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.ward_cost(), _ => None,
        }).unwrap();
        let branches = ward.as_one_of().expect("ward remains a disjunction");
        assert_eq!(branches.len(), 2);
        assert!(branches.iter().all(|branch| branch.as_all().is_some_and(|costs| costs.len() == 1)));
        assert_eq!(branches.iter().filter(|branch| branch.mana_cost().is_some()).count(), 1);
        let rendered = definition.canonical_text.to_ascii_lowercase();
        assert!(rendered.contains("additional cost"));
        assert!(rendered.contains("ward"));
        assert!(rendered.matches(" or ").count() >= 2, "both alternatives must be rendered: {rendered}");
    }
}

#[test]
fn casting_pays_exactly_one_branch_in_addition_to_printed_hybrid_mana() {
    for definition in definitions() {
        for (discard, funding) in [
            (true, ManaSymbol::Black), (false, ManaSymbol::Black),
            (true, ManaSymbol::Green), (false, ManaSymbol::Green),
        ] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let resource = hand_card(&mut game, A);
            mana(&mut game, A, funding, 7);
            let mut choices = Choices::new(discard, Some(resource));
            choices.hybrid = funding;
            cast(&mut game, spell, &mut choices, &mut TriggerQueue::new()).unwrap();
            assert_eq!(choices.branch_prompts, 1);
            assert_eq!(choices.ward_prompts, 0, "casting must not pay its own ward");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), if discard { 4 } else { 2 });
            assert_eq!(game.player(A).unwrap().hand.contains(&resource), !discard);
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(discard));
            assert_eq!(game.stack.len(), 1);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name == definition.card.name));
        }
    }
}

#[test]
fn casting_failure_multiple_selection_and_cancellation_leave_no_partial_payment() {
    for definition in definitions() {
        // Neither additional alternative can be paid with only the printed mana.
        let mut unfunded = game();
        let spell = unfunded.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut unfunded, A, ManaSymbol::Black, 3);
        assert!(!compute_legal_actions(&unfunded, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)));
        assert!(unfunded.player(A).unwrap().graveyard.is_empty());
        assert_eq!(unfunded.player(A).unwrap().mana_pool.total(), 3);
        for (discard, invalid_multiple) in [(true, false), (false, false), (true, true)] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let resource = hand_card(&mut game, A);
            mana(&mut game, A, ManaSymbol::Black, 7);
            let before = game.player(A).unwrap().mana_pool.clone();
            let mut choices = Choices::new(discard, Some(resource));
            choices.cancel_mana = !invalid_multiple;
            choices.invalid_multiple = invalid_multiple;
            let result = cast(&mut game, spell, &mut choices, &mut TriggerQueue::new());
            if invalid_multiple { assert!(result.is_err(), "two selected prices must be rejected"); }
            else { assert!(choices.cancelled, "cancel the actual payment transaction"); }
            assert!(game.stack_is_empty());
            assert!(game.player(A).unwrap().hand.contains(&spell));
            assert!(game.player(A).unwrap().hand.contains(&resource));
            assert!(game.player(A).unwrap().graveyard.is_empty());
            assert_eq!(game.player(A).unwrap().mana_pool, before);
        }
    }
}

fn target_titania(game: &mut GameState, choices: &mut Choices) {
    let spell = compile_to_runtime_definition("Targeting spell", "Mana cost: {U}\nType: Instant\nTap target creature.", false).unwrap();
    let id = game.create_object_from_definition(&spell, A, Zone::Hand);
    mana(game, A, ManaSymbol::Blue, 1);
    let mut queue = TriggerQueue::new();
    cast(game, id, choices, &mut queue).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    assert_eq!(game.stack.len(), 2, "the targeting spell plus its real ward trigger");
}

#[test]
fn ward_each_branch_charges_targeting_controller_once_and_preserves_unchosen_resources() {
    for definition in definitions() {
        for discard in [true, false] {
            let mut game = game();
            let target = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let resource = hand_card(&mut game, A);
            let protected_resource = hand_card(&mut game, B);
            mana(&mut game, A, ManaSymbol::Black, 4);
            mana(&mut game, B, ManaSymbol::Black, 9);
            let mut choices = Choices::new(discard, Some(resource));
            choices.target = Some(target);
            target_titania(&mut game, &mut choices);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 4, "targeting spell cost is separate");
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(choices.ward_prompts, 1);
            assert_eq!(choices.branch_prompts, 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), if discard { 4 } else { 2 });
            assert_eq!(game.player(A).unwrap().hand.contains(&resource), !discard);
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(discard));
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 9);
            assert_eq!(game.player(B).unwrap().hand, vec![protected_resource]);
            assert_eq!(game.stack.len(), 1);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.is_tapped(target));
        }
    }
}

#[test]
fn ward_decline_failure_and_mana_cancel_counter_without_spending_either_price() {
    for definition in definitions() {
        for (funded, decline, cancel) in [(true, true, false), (false, false, false), (true, false, true)] {
            let mut game = game();
            let target = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let resource = funded.then(|| hand_card(&mut game, A));
            if funded { mana(&mut game, A, ManaSymbol::Black, 4); }
            let mut choices = Choices::new(false, resource);
            choices.target = Some(target);
            target_titania(&mut game, &mut choices);
            choices.pay_ward = !decline;
            choices.cancel_mana = cancel;
            let pool = game.player(A).unwrap().mana_pool.clone();
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(choices.ward_prompts, 1);
            if cancel { assert!(choices.cancelled); }
            assert!(game.stack_is_empty(), "unpaid ward counters the targeting spell");
            assert!(!game.is_tapped(target));
            assert_eq!(game.player(A).unwrap().mana_pool, pool);
            if let Some(resource) = resource { assert!(game.player(A).unwrap().hand.contains(&resource)); }
            assert_eq!(game.player(A).unwrap().graveyard.len(), 1, "only the countered spell moved");
        }
    }
}

#[test]
fn unavailable_unchosen_prices_do_not_block_casting_or_ward_payment() {
    for definition in definitions() {
        for discard in [true, false] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            let resource = discard.then(|| hand_card(&mut game, A));
            mana(&mut game, A, ManaSymbol::Black, if discard { 3 } else { 5 });
            let mut choices = Choices::new(discard, resource);
            cast(&mut game, spell, &mut choices, &mut TriggerQueue::new()).unwrap();
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(discard));

            let mut game = self::game();
            let target = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let resource = discard.then(|| hand_card(&mut game, A));
            if !discard { mana(&mut game, A, ManaSymbol::Black, 2); }
            let mut choices = Choices::new(discard, resource);
            choices.target = Some(target);
            target_titania(&mut game, &mut choices);
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.stack.len(), 1, "the sole payable ward alternative suffices");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(discard));
            assert_eq!(choices.ward_prompts, 1);
        }
    }
}

#[test]
fn blue_only_funding_cannot_substitute_for_either_printed_hybrid_color() {
    for definition in definitions() {
        for with_discard_resource in [true, false] {
            let mut game = game();
            let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
            if with_discard_resource { hand_card(&mut game, A); }
            mana(&mut game, A, ManaSymbol::Blue, 7);
            let before_pool = game.player(A).unwrap().mana_pool.clone();
            let before_hand = game.player(A).unwrap().hand.clone();
            assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
                "neither discard nor extra generic mana replaces the printed B/G pip");
            assert_eq!(game.player(A).unwrap().mana_pool, before_pool);
            assert_eq!(game.player(A).unwrap().hand, before_hand);
            assert!(game.player(A).unwrap().graveyard.is_empty());
            assert!(game.stack_is_empty());
        }
    }
}

#[test]
fn illegal_target_before_announcement_offers_no_cast_payment_or_ward_trigger() {
    for definition in definitions() {
        let mut game = game();
        let target = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let departed = game.move_object_by_effect(target, Zone::Graveyard).unwrap();
        let spell = compile_to_runtime_definition("Targeting spell", "Mana cost: {U}\nType: Instant\nTap target creature.", false).unwrap();
        let spell = game.create_object_from_definition(&spell, A, Zone::Hand);
        let resource = hand_card(&mut game, A);
        mana(&mut game, A, ManaSymbol::Blue, 1);
        mana(&mut game, A, ManaSymbol::Black, 4);
        let before_pool = game.player(A).unwrap().mana_pool.clone();
        let before_hand = game.player(A).unwrap().hand.clone();
        assert!(game.object(target).is_none(), "the original target identity has departed");
        assert!(game.battlefield.is_empty(), "there are no legal creature targets");
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)));
        let mut queue = TriggerQueue::new();
        let mut choices = Choices::new(true, Some(resource));
        choices.target = Some(target);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        assert!(queue.is_empty());
        assert!(game.stack_is_empty());
        assert_eq!(choices.ward_prompts, 0);
        assert_eq!(choices.branch_prompts, 0);
        assert_eq!(game.player(A).unwrap().mana_pool, before_pool);
        assert_eq!(game.player(A).unwrap().hand, before_hand);
        assert!(game.player(A).unwrap().graveyard.is_empty());
        assert_eq!(game.player(B).unwrap().graveyard, vec![departed]);
    }
}

#[test]
fn target_departure_after_ward_queues_preserves_payment_then_invalidates_the_spell_effect() {
    for definition in definitions() {
        for discard in [true, false] {
            let mut game = game();
            let target = game.create_object_from_definition(&definition, B, Zone::Battlefield);
            let resource = hand_card(&mut game, A);
            let protected_resource = hand_card(&mut game, B);
            mana(&mut game, A, ManaSymbol::Black, 4);
            mana(&mut game, B, ManaSymbol::Black, 9);
            let mut choices = Choices::new(discard, Some(resource));
            choices.target = Some(target);
            target_titania(&mut game, &mut choices);
            let departed = game.move_object_by_effect(target, Zone::Graveyard).unwrap();
            assert_eq!(game.stack.len(), 2, "departure does not remove the queued ward obligation");
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(choices.ward_prompts, 1);
            assert_eq!(choices.branch_prompts, 1);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), if discard { 4 } else { 2 });
            assert_eq!(game.player(A).unwrap().hand.contains(&resource), !discard);
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(discard));
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 9);
            assert_eq!(game.player(B).unwrap().hand, vec![protected_resource]);
            assert_eq!(game.player(B).unwrap().graveyard, vec![departed]);
            assert_eq!(game.stack.len(), 1, "paying ward leaves the spell to check its own target legality");
            // A returned incarnation is a distinct object and must not become
            // the old spell's target merely because it is the same physical card.
            let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap();
            assert_ne!(returned, target);
            assert!(!game.is_tapped(returned));
            let after_ward_pool = game.player(A).unwrap().mana_pool.clone();
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert!(game.stack_is_empty());
            assert!(!game.is_tapped(returned), "the stale-target spell has no legal tap effect");
            assert_eq!(game.player(A).unwrap().graveyard.len(), 1 + usize::from(discard));
            assert_eq!(game.player(A).unwrap().mana_pool, after_ward_pool);
            assert_eq!(choices.ward_prompts, 1, "no new ward obligation for an unchosen incarnation");
            assert_eq!(game.player(B).unwrap().mana_pool.total(), 9);
            assert_eq!(game.player(B).unwrap().hand, vec![protected_resource]);
        }
    }
}

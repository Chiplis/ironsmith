//! Custom reminder compositions through existing optional-cost and next-spell operations.
//! Parent runs validation; this module does not introduce gameplay rules.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;


const A: PlayerId = PlayerId::from_index(0);

#[derive(Default)]
struct Choices {
    pay: Vec<&'static str>,
    seen_optional_costs: Vec<String>,
    pay_index: Option<usize>,
}

impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            self.seen_optional_costs
                .extend(ctx.options.iter().map(|option| option.description.clone()));
            if let Some(index) = self.pay_index {
                return ctx.options.iter().filter(|option| option.legal && option.index == index).map(|option| option.index).collect();
            }
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
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || compile_to_runtime_definition(name, text, false)).unwrap_or_else(|error| panic!("{name}: {error}"))
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
fn optional_mana_copy_cost_can_be_paid_or_declined_without_replicate_identity() {
    // Quiet Meditation's authored Reverberate {2} is a one-time payment.
    let definition = fixture("Quiet Meditation", "Mana cost: {U}\nType: Sorcery\nAs an additional cost to cast this spell, you may pay {2}. When you do, copy this spell.\nDraw a card.");
    assert_eq!(definition.optional_costs.len(), 1);
    let cost = &definition.optional_costs[0];
    assert!(!cost.repeatable);
    assert_eq!(cost.kind.official_keyword(), None);
    for paid in [false, true] {
        let mut game = game();
        mana(&mut game, ManaSymbol::Blue, 1);
        mana(&mut game, ManaSymbol::Colorless, 2);
        let mut choices = Choices { pay: if paid { vec!["additional"] } else { vec![] }, ..Default::default() };
        cast(&mut game, &definition, &mut choices);
        assert_eq!(game.stack.len(), if paid { 2 } else { 1 }, "only the paid cast has a copy trigger");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if paid { 0 } else { 2 });
        if paid {
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.stack.len(), 2, "resolving the trigger adds exactly one copy");
        }
    }
    // An official replicate payment remains separately identified and repeatable.
    let official = fixture("Official copy payment", "Mana cost: {U}\nType: Sorcery\nReplicate {2}\nDraw a card.");
    assert!(official.optional_costs.iter().any(|cost| cost.kind.official_keyword() == Some("replicate") && cost.repeatable));
}

#[test]
fn existing_next_spell_reductions_stack_on_one_cast_and_are_consumed_even_when_excessive() {
    use ironsmith::effects::{EffectContext, execute_effect};
    let catalyze = fixture("Catalyze operation", "Type: Sorcery\nThe next spell you cast this turn costs {1} less to cast.");
    let priced = fixture("Next spell", "Mana cost: {1}{U}\nType: Instant\nYou gain 1 life.");
    let mut game = game();
    let source = game.create_object_from_definition(&catalyze, A, Zone::Stack);
    for _ in 0..2 {
        for effect in catalyze.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new_default(source, A)).unwrap();
        }
    }
    assert_eq!(game.effect_store.temporary_spell_cost_reductions.len(), 2);
    mana(&mut game, ManaSymbol::Blue, 2);
    mana(&mut game, ManaSymbol::Colorless, 1);
    cast(&mut game, &priced, &mut Choices::default());
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 2, "both reductions apply to the same spell, with excess lost");
    assert!(game.effect_store.temporary_spell_cost_reductions.iter().all(|effect| effect.remaining_uses == 0));
    game.turn.priority_player = Some(A);
    cast(&mut game, &priced, &mut Choices::default());
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "the following spell pays full cost");
}

#[test]
fn optional_tap_power_reduction_pays_total_cost_and_preserves_identity() {
    let definition = fixture("Power payment", "Mana cost: {3}{U}\nType: Instant\nAs an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.\nYou gain 1 life.");
    let body = fixture("Resource", "Type: Creature — Ox\nPower/Toughness: 3/3");
    let tax = fixture("Tax", "Type: Enchantment\nSpells cost {2} more to cast.");
    assert!(definition.optional_costs.iter().all(|cost| cost.kind.official_keyword().is_none()));
    assert!(definition.alternative_casts.is_empty(), "an additional cost is compatible with alternative prices and grants no casting identity");
    for paid in [false, true] {
        let mut game = game();
        let creature = game.create_object_from_definition(&body, A, Zone::Battlefield);
        game.create_object_from_definition(&tax, A, Zone::Battlefield);
        mana(&mut game, ManaSymbol::Blue, 1);
        mana(&mut game, ManaSymbol::Colorless, 5);
        let mut choices = Choices { pay: if paid { vec!["additional"] } else { vec![] }, ..Default::default() };
        cast(&mut game, &definition, &mut choices);
        assert_eq!(game.is_tapped(creature), paid);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if paid { 3 } else { 0 }, "power reduces the entire increased cost");
        assert!(!game.is_harnessed(creature), "payment does not set the distinct permanent designation");
        assert_eq!(game.stack.len(), 1);
    }
}

#[test]
fn tap_power_reduction_is_visible_to_legality_and_uses_selected_power_not_creature_count() {
    let definition = fixture("Reduced cast", "Mana cost: {3}{U}\nType: Instant\nAs an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.\nYou gain 1 life.");
    let body = fixture("Resource", "Type: Creature — Ox\nPower/Toughness: 3/3");
    let mut game = game();
    let creature = game.create_object_from_definition(&body, A, Zone::Battlefield);
    mana(&mut game, ManaSymbol::Blue, 1);
    let mut choices = Choices { pay: vec!["additional"], ..Default::default() };
    cast(&mut game, &definition, &mut choices);
    assert!(game.is_tapped(creature));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}

#[test]
fn scoped_optional_tap_metric_ignores_a_second_unrelated_tap_cost_even_when_declined() {
    let definition = fixture("Two tap prices", "Mana cost: {6}{U}\nType: Instant\nAs an additional cost to cast this spell, tap an untapped green creature you control.\nAs an additional cost to cast this spell, you may tap an untapped red creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.\nYou gain 1 life.");
    let red = fixture("Optional resource", "Mana cost: {R}\nType: Creature — Ox\nPower/Toughness: 3/3");
    let green = fixture("Mandatory resource", "Mana cost: {G}\nType: Creature — Elf\nPower/Toughness: 7/7");
    for paid in [false, true] {
        let mut game = game();
        let optional = game.create_object_from_definition(&red, A, Zone::Battlefield);
        let mandatory = game.create_object_from_definition(&green, A, Zone::Battlefield);
        mana(&mut game, ManaSymbol::Blue, 1);
        mana(&mut game, ManaSymbol::Colorless, 6);
        let mut choices = Choices { pay: if paid { vec!["additional"] } else { vec![] }, ..Default::default() };
        cast(&mut game, &definition, &mut choices);
        assert!(game.is_tapped(mandatory));
        assert_eq!(game.is_tapped(optional), paid);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if paid { 3 } else { 0 }, "the unrelated seven-power tap must not reduce the spell's cost");
    }
}

#[test]
fn the_tapped_discount_creature_cannot_also_pay_the_remaining_mana_cost() {
    let definition = fixture("Joint payment", "Mana cost: {4}\nType: Instant\nAs an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.\nYou gain 1 life.");
    let body = fixture("Mana resource", "Type: Creature — Elf\nPower/Toughness: 3/3\n{T}: Add {C}.");
    let mut game = game();
    let creature = game.create_object_from_definition(&body, A, Zone::Battlefield);
    game.remove_summoning_sickness(creature);
    let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
    assert!(!compute_legal_actions(&game, A).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)), "one tap cannot both earn three generic discount and produce the fourth mana");
    mana(&mut game, ManaSymbol::Colorless, 1);
    let mut choices = Choices { pay: vec!["additional"], ..Default::default() };
    // The original uncast hand card is retained for the negative assertion;
    // cast() creates a separate identical card and follows its real menu.
    cast(&mut game, &definition, &mut choices);
    assert!(game.is_tapped(creature));
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
}

#[test]
fn paying_a_different_optional_cost_does_not_trigger_the_copy_payment() {
    let definition = fixture("Distinct optional payments", "Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may pay {2}. When you do, copy this spell.\nAs an additional cost to cast this spell, you may pay {1}. When you do, you gain 1 life.\nYou gain 1 life.");
    assert_ne!(definition.optional_costs[0].cost_ref(), definition.optional_costs[1].cost_ref());
    let mut game = game();
    mana(&mut game, ManaSymbol::Blue, 1);
    mana(&mut game, ManaSymbol::Colorless, 1);
    let mut choices = Choices { pay_index: Some(1), ..Default::default() };
    cast(&mut game, &definition, &mut choices);
    assert_eq!(game.stack.len(), 2, "only the separately paid life-gain cost triggers");
    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
    assert_eq!(game.stack.len(), 1, "no copy was made from a different optional payment");
    assert_eq!(game.player(A).unwrap().life, 21);
}

#[test]
fn payment_linked_power_reduction_respects_minimum_total_mana_cost() {
    let definition = fixture("Minimum payment", "Mana cost: {4}\nType: Instant\nAs an additional cost to cast this spell, you may tap an untapped creature you control. This spell costs {X} less to cast, where X is the power of the creature tapped this way.\nYou gain 1 life.");
    let body = fixture("Resource", "Type: Creature\nPower/Toughness: 4/4");
    let floor = fixture("Minimum", "Type: Artifact\nAs long as this artifact is untapped, each spell that would cost less than three mana to cast costs three mana to cast.");
    let mut game = game();
    game.create_object_from_definition(&body, A, Zone::Battlefield);
    game.create_object_from_definition(&floor, A, Zone::Battlefield);
    mana(&mut game, ManaSymbol::Colorless, 3);
    cast(&mut game, &definition, &mut Choices { pay: vec!["additional"], ..Default::default() });
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0, "the minimum applies after the optional tap's power reduction");
}

#[test]
fn scoped_cost_attribution_preserves_instance_and_rejects_ambiguous_multiple_costs() {
    let text = "Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may pay {2}. When you do, copy this spell.\nYou gain 1 life.";
    let mut definition = fixture("Scoped copy payment", text);
    let original_reference = definition.optional_costs[0].cost_ref();
    assert!(definition.attribute_custom_keyword_payment("MSEM Echo payment"));
    let attributed = definition.optional_costs[0].cost_ref();
    assert_eq!(attributed.discriminator, original_reference.discriminator);
    assert!(attributed.matches_query(&original_reference));
    assert_eq!(attributed.kind.official_keyword(), None);
    let mut paid = ironsmith_core::OptionalCostsPaid::default();
    paid.mark_label_paid(attributed);
    assert!(paid.paid_custom_keyword("MSEM Echo payment"));
    assert!(!paid.paid_custom_keyword("replicate"));
    assert!(paid.was_paid_label(original_reference));
    let mut game = game();
    mana(&mut game, ManaSymbol::Blue, 1);
    mana(&mut game, ManaSymbol::Colorless, 2);
    let mut choices = Choices { pay: vec!["additional"], ..Default::default() };
    cast(&mut game, &definition, &mut choices);
    assert_eq!(game.stack.len(), 2, "attributing the cost does not break its exact paid-label cast trigger");
    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
    assert_eq!(game.stack.len(), 2);
    let mut ambiguous = fixture("Two custom payment candidates", "Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may pay {2}. When you do, copy this spell.\nAs an additional cost to cast this spell, you may pay {1}. When you do, you gain 1 life.\nYou gain 1 life.");
    assert!(!ambiguous.attribute_custom_keyword_payment("MSEM Echo payment"));
    assert!(ambiguous.optional_costs.iter().all(|cost| cost.cost_ref().custom_keyword.is_none()));
}

#[test]
fn legacy_payment_reference_json_roundtrips_without_new_empty_metadata() {
    let original = ironsmith_core::OptionalCostRef::new(ironsmith_core::OptionalCostKind::Additional);
    let encoded = serde_json::to_value(&original).unwrap();
    assert!(encoded.get("custom_keyword").is_none());
    assert!(encoded.get("custom_keyword_attributable").is_none());
    let decoded: ironsmith_core::OptionalCostRef = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
}

#[test]
fn identical_mana_prices_are_independent_optional_payment_instances() {
    let definition = fixture("Identical separate prices", "Mana cost: {U}\nType: Instant\nAs an additional cost to cast this spell, you may pay {2}. When you do, copy this spell.\nAs an additional cost to cast this spell, you may pay {2}. When you do, you gain 1 life.\nYou gain 1 life.");
    assert_ne!(definition.optional_costs[0].cost_ref(), definition.optional_costs[1].cost_ref());
    for chosen in [0, 1] {
        let mut game = game();
        mana(&mut game, ManaSymbol::Blue, 1);
        mana(&mut game, ManaSymbol::Colorless, 2);
        let mut choices = Choices { pay_index: Some(chosen), ..Default::default() };
        cast(&mut game, &definition, &mut choices);
        assert_eq!(game.stack.len(), 2, "only the chosen payment's trigger is created");
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.stack.len(), if chosen == 0 { 2 } else { 1 });
        assert_eq!(game.player(A).unwrap().life, if chosen == 0 { 20 } else { 21 });
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
    let mut ambiguous = definition;
    assert!(!ambiguous.attribute_custom_keyword_payment("MSEM Echo payment"));
}

#[test]
fn mystical_echo_paid_copy_resolves_as_a_token_permanent_without_replicate_identity() {
    let definition = fixture("Mystical Echo", "Mana cost: {2}{G}\nType: Creature — Cat Reflection\nPower/Toughness: 3/3\nAs an additional cost to cast this spell, you may pay {2}{G}. When you do, copy this spell.\nMystical Echo gets +1/+1 for each other Reflection you control.\nWhenever you discard a card, Reflections you control gain trample until end of turn.");
    let mut game = game();
    mana(&mut game, ManaSymbol::Green, 2);
    mana(&mut game, ManaSymbol::Colorless, 4);
    let mut choices = Choices { pay: vec!["additional"], ..Default::default() };
    cast(&mut game, &definition, &mut choices);
    assert_eq!(game.stack.len(), 2);
    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
    assert_eq!(game.stack.len(), 2, "the cast trigger created exactly one permanent spell copy");
    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
    assert_eq!(game.battlefield.len(), 1);
    assert_eq!(game.object(game.battlefield[0]).unwrap().kind, ironsmith::object::ObjectKind::Token);
    resolve_stack_entry_with(&mut game, &mut choices).unwrap();
    assert_eq!(game.battlefield.len(), 2);
    assert_eq!(game.battlefield.iter().filter(|&&id| game.object(id).unwrap().kind == ironsmith::object::ObjectKind::Token).count(), 1);
    assert!(definition.optional_costs.iter().all(|cost| cost.kind.official_keyword().is_none()));
}

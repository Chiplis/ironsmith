use super::*;
use crate::ability::{
    Ability, AbilityKind, ActivatedAbility, ManaPaymentPredicate, ManaPaymentPurpose,
    ManaSpendPayload, ManaUsageRestriction, ManaUsageSubtypeRequirement, RestrictedManaUnit,
};
use crate::cards::CardDefinitionBuilder;
use crate::cards::definitions::{
    basic_mountain, basic_swamp, blood_celebrant, command_tower, ornithopter, phyrexian_tower,
    wall_of_roots, yawgmoth_thran_physician,
};
use crate::cards::tokens::treasure_token_definition;
use crate::color::Color;
use crate::cost::TotalCost;
use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
use crate::game_state::Phase;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

fn setup_game() -> GameState {
    crate::tests::test_helpers::setup_two_player_game()
}

#[test]
fn spell_payment_uses_one_authoritative_plan_and_commits_it_atomically() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let land = CardDefinitionBuilder::new(CardId::new(), "Test Mountain")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ))
        .build();
    let land = game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Red]);
    let spell = CardDefinitionBuilder::new(CardId::new(), "Unified Payment Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(cost.clone())
        .build();
    let hand_spell = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let stack_spell = game
        .move_object(
            hand_spell,
            Zone::Stack,
            crate::events::cause::EventCause::effect(),
        )
        .expect("spell should move to the stack during announcement");
    let mut pending = PendingCast::new(
        stack_spell,
        Zone::Hand,
        alice,
        crate::provenance::ProvNodeId::default(),
        CastStage::PayingMana,
        None,
        Vec::new(),
        crate::alternative_cast::CastingMethod::Normal,
        crate::cost::OptionalCostsPaid::default(),
        None,
        stack_spell,
    );
    pending.mana_cost_to_pay = Some(cost);
    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let progress = prompt_spell_mana_ability_window(
        &mut game,
        &mut trigger_queue,
        &mut state,
        pending,
        &mut dm,
    )
    .expect("spell should produce a payment proposal");
    let crate::decisions::context::DecisionContext::ManaPayment(context) = (match progress {
        GameProgress::NeedsDecisionCtx(context) => context,
        other => panic!("expected one mana-payment decision, got {other:?}"),
    }) else {
        panic!("spell payment must use the authoritative whole-cost decision");
    };
    assert_eq!(context.plan.mana_ability_steps.len(), 1);
    assert_eq!(context.plan.mana_ability_steps[0].source, land);

    let response = crate::mana_payment::ManaPaymentResponse::Confirm {
        plan_id: context.plan.id,
        request_hash: context.plan.request_hash,
    };
    apply_mana_payment_plan_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    )
    .expect("the authoritative payment plan should commit");

    assert!(game.is_tapped(land));
    assert_eq!(game.player(alice).expect("player").mana_pool.red, 0);
    assert!(state.pending_cast.is_none());
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == stack_spell)
    );
}

#[test]
fn mana_payment_waits_until_selected_from_the_total_cost_order() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let land_definition = CardDefinitionBuilder::new(CardId::new(), "Ordering Mountain")
        .card_types(vec![CardType::Land])
        .with_ability(Ability::mana(
            TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Red],
        ))
        .build();
    let land = game.create_object_from_definition(&land_definition, alice, Zone::Battlefield);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Red]);
    let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Ordered Payment Probe")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(cost.clone())
        .build();
    let hand_spell = game.create_object_from_definition(&spell_definition, alice, Zone::Hand);
    let stack_spell = game
        .move_object(
            hand_spell,
            Zone::Stack,
            crate::events::cause::EventCause::effect(),
        )
        .expect("spell should move to the stack during announcement");
    let mut pending = PendingCast::new(
        stack_spell,
        Zone::Hand,
        alice,
        crate::provenance::ProvNodeId::default(),
        CastStage::ChoosingNextCost,
        None,
        Vec::new(),
        crate::alternative_cast::CastingMethod::Normal,
        crate::cost::OptionalCostsPaid::default(),
        None,
        stack_spell,
    );
    pending.mana_cost_to_pay = Some(cost);
    pending.remaining_cost_steps = vec![ActivationCostStep::Cost(crate::costs::Cost::life(1))];
    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;

    let progress = continue_spell_next_cost_or_finalize(
        &mut game,
        &mut trigger_queue,
        &mut state,
        pending,
        &mut dm,
    )
    .expect("the authoritative proposal must precede every cost payment");
    let payment = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::ManaPayment(context),
        ) => context,
        other => panic!("expected a pre-cost mana-payment proposal, got {other:?}"),
    };
    assert!(
        !game.is_tapped(land),
        "previewing the plan must be side-effect free"
    );
    assert_eq!(game.player(alice).expect("player").mana_pool.total(), 0);

    let progress = apply_mana_payment_plan_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        },
        &mut dm,
    )
    .expect("confirming should prepare mana sources before any costs are paid");
    let options = match progress {
        GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(context),
        ) => context.options,
        other => panic!("expected a total-cost ordering choice, got {other:?}"),
    };
    assert_eq!(options.len(), 2);
    assert!(
        game.is_tapped(land),
        "CR 601.2g source activation happens before the cost-order choice"
    );
    assert_eq!(game.player(alice).expect("player").mana_pool.total(), 1);

    let mana_choice = options
        .iter()
        .find(|option| option.description.starts_with("Mana:"))
        .map(|option| option.index)
        .expect("mana should be one of the remaining cost components");
    let progress = apply_next_cost_choice_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        mana_choice,
        &mut dm,
    )
    .expect("selecting mana should commit the already-authorized payment");
    assert!(!matches!(
        progress,
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::ManaPayment(_))
    ));
    assert!(game.is_tapped(land));
    assert_eq!(game.player(alice).expect("player").mana_pool.total(), 0);
}

fn add_test_mana_from_source(
    game: &mut GameState,
    player: PlayerId,
    name: &str,
    snow: bool,
    symbol: ManaSymbol,
) -> ObjectId {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]);
    if snow {
        builder = builder.supertypes(vec![Supertype::Snow]);
    }
    let source = game.create_object_from_definition(&builder.build(), player, Zone::Battlefield);
    let snapshot = ObjectSnapshot::from_object(game.object(source).expect("mana source"), game);
    game.player_mut(player)
        .expect("player")
        .add_unrestricted_mana(symbol, source, Some(snapshot));
    source
}

#[test]
fn i006_generic_cost_reduction_never_reduces_a_snow_pip() {
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(3), ManaSymbol::Snow]);
    let reduced = cost.reduce_generic(99);
    assert_eq!(reduced.pips(), &[vec![ManaSymbol::Snow]]);
}

#[test]
fn i006_bulk_snow_payment_rejects_nonsnow_mana_and_accepts_snow_mana() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    add_test_mana_from_source(
        &mut game,
        alice,
        "Ordinary Colorless Source",
        false,
        ManaSymbol::Colorless,
    );
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Snow]);

    assert!(!game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert!(!game.try_pay_mana_cost_with_reason(
        alice,
        None,
        &cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert_eq!(game.player(alice).expect("player").mana_pool.colorless, 1);

    add_test_mana_from_source(&mut game, alice, "Snow Blue Source", true, ManaSymbol::Blue);
    assert!(game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert!(game.try_pay_mana_cost_with_reason(
        alice,
        None,
        &cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert_eq!(game.player(alice).expect("player").mana_pool.blue, 0);
    assert_eq!(game.player(alice).expect("player").mana_pool.colorless, 1);
}

#[test]
fn i006_mana_remembers_a_continuously_snow_source_after_the_effect_ends() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Temporarily Snow Land")
        .card_types(vec![CardType::Land])
        .taps_for(ManaSymbol::Green)
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let ability_index = game
        .object(source)
        .expect("mana source")
        .abilities
        .iter()
        .position(Ability::is_mana_ability)
        .expect("mana ability");
    let effect_id = game.effect_store.continuous_effects.add_effect(
        crate::continuous::ContinuousEffect::new(
            source,
            alice,
            crate::continuous::EffectTarget::Specific(source),
            crate::continuous::Modification::AddSupertypes(vec![Supertype::Snow]),
        )
        .until(crate::effect::Until::EndOfTurn),
    );
    assert!(game.current_has_supertype(source, Supertype::Snow));

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        source,
        ability_index,
        &mut dm,
    )
    .expect("snow source mana ability should activate");
    game.effect_store
        .continuous_effects
        .remove_effect(effect_id);
    assert!(!game.current_has_supertype(source, Supertype::Snow));

    let snow_cost = ManaCost::from_symbols(vec![ManaSymbol::Snow]);
    assert!(game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &snow_cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert!(game.try_pay_mana_cost_with_reason(
        alice,
        None,
        &snow_cost,
        0,
        crate::costs::PaymentReason::Other,
    ));
}

#[test]
fn ordered_graveyard_cost_candidates_only_offer_the_top_matching_card() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Ordered Cost Source")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let creature = |name| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build()
    };
    let noncreature = |name| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build()
    };
    let lower_creature =
        game.create_object_from_definition(&creature("Lower Creature"), alice, Zone::Graveyard);
    let _middle_noncreature = game.create_object_from_definition(
        &noncreature("Middle Noncreature"),
        alice,
        Zone::Graveyard,
    );
    let top_creature =
        game.create_object_from_definition(&creature("Top Creature"), alice, Zone::Graveyard);
    let _actual_top = game.create_object_from_definition(
        &noncreature("Actual Top Noncreature"),
        alice,
        Zone::Graveyard,
    );

    let mut filter = crate::target::ObjectFilter::creature();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(crate::target::PlayerFilter::You);

    assert_eq!(
        get_legal_cost_choice_objects(&game, alice, source_id, &filter, Zone::Graveyard, true,),
        vec![top_creature]
    );
    let ordinary =
        get_legal_cost_choice_objects(&game, alice, source_id, &filter, Zone::Graveyard, false);
    assert!(ordinary.contains(&lower_creature));
    assert!(ordinary.contains(&top_creature));
}

#[test]
fn opponent_owned_exile_cost_candidates_exclude_the_activating_players_cards() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = CardDefinitionBuilder::new(CardId::new(), "Processor Source")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let material = |name| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build()
    };
    let alice_exiled =
        game.create_object_from_definition(&material("Alice Exiled"), alice, Zone::Exile);
    let bob_exiled = game.create_object_from_definition(&material("Bob Exiled"), bob, Zone::Exile);

    let filter = crate::target::ObjectFilter::default()
        .in_zone(Zone::Exile)
        .owned_by(crate::target::PlayerFilter::Opponent);

    let candidates =
        get_legal_cost_choice_objects(&game, alice, source_id, &filter, Zone::Exile, false);
    assert_eq!(candidates, vec![bob_exiled]);
    assert!(!candidates.contains(&alice_exiled));
}

fn arena_style_land_definition() -> crate::cards::CardDefinition {
    let ability = Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                crate::costs::Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])),
                crate::costs::Cost::tap(),
                crate::costs::Cost::effect(crate::effects::ExertCostEffect::new("Exert this land")),
            ]),
            effects: ResolutionProgram::default(),
            choices: Vec::new(),
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: Vec::new(),
            mana_output: Some(vec![ManaSymbol::Red, ManaSymbol::Red]),
            activation_condition: None,
            mana_usage_restrictions: vec![ManaUsageRestriction::CastSpellWithManaBonus {
                filter: crate::target::ObjectFilter::creature(),
                condition: crate::ability::ManaSpendBonusCondition::IfThatManaIsSpentOn,
                grant_uncounterable: false,
                enters_with_counters: Vec::new(),
                granted_abilities: vec![(
                    StaticAbilityId::Haste,
                    crate::ability::ManaSpendAbilityGrantDuration::UntilEndOfTurn,
                )],
                granted_keywords: Vec::new(),
            }],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    CardDefinitionBuilder::new(CardId::new(), "Arena Style Land")
        .card_types(vec![CardType::Land])
        .with_ability(ability)
        .build()
}

fn restricted_mana_ability_index(game: &GameState, source: ObjectId) -> usize {
    game.object(source)
        .expect("source should exist")
        .abilities
        .iter()
        .enumerate()
        .find_map(|(idx, ability)| {
            if matches!(
                &ability.kind,
                AbilityKind::Activated(activated)
                    if !activated.mana_usage_restrictions.is_empty()
            ) {
                Some(idx)
            } else {
                None
            }
        })
        .expect("source should have a restricted mana ability")
}

#[test]
fn test_single_flexible_mana_source_cannot_pay_two_colored_pips() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(&treasure_token_definition(), alice, Zone::Battlefield);
    let two_color_spell = CardDefinitionBuilder::new(CardId::new(), "Two Color Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::White],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_definition(&two_color_spell, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "one any-color mana source should not make a two-colored-pip spell legal"
    );
}

#[test]
fn test_single_flexible_mana_source_can_pay_one_colored_pip() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(&treasure_token_definition(), alice, Zone::Battlefield);
    let one_color_spell = CardDefinitionBuilder::new(CardId::new(), "One Color Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_definition(&one_color_spell, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "one any-color mana source should still make a one-colored-pip spell legal"
    );
}

#[test]
fn test_tapped_lands_do_not_make_spell_castable() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let plains_one = game.create_object_from_definition(
        &crate::cards::definitions::basic_plains(),
        alice,
        Zone::Battlefield,
    );
    let plains_two = game.create_object_from_definition(
        &crate::cards::definitions::basic_plains(),
        alice,
        Zone::Battlefield,
    );
    game.tap(plains_one);
    game.tap(plains_two);

    let creature = CardDefinitionBuilder::new(CardId::new(), "Two Mana White Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .build();
    let spell_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "two tapped Plains should not make a {{1}}{{W}} creature legal to cast"
    );
}

#[test]
fn test_tapped_lands_plus_one_floating_mana_do_not_make_two_mana_spell_castable() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let plains_one = game.create_object_from_definition(
        &crate::cards::definitions::basic_plains(),
        alice,
        Zone::Battlefield,
    );
    let plains_two = game.create_object_from_definition(
        &crate::cards::definitions::basic_plains(),
        alice,
        Zone::Battlefield,
    );
    game.tap(plains_one);
    game.tap(plains_two);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::White, 1);

    let creature = CardDefinitionBuilder::new(CardId::new(), "Two Mana White Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .build();
    let spell_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "one floating white and tapped lands should not make a {{1}}{{W}} creature legal"
    );
}

#[test]
fn stacked_activated_ability_preserves_mana_usage_restrictions() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardDefinitionBuilder::new(CardId::new(), "Sarkhan Test")
        .card_types(vec![CardType::Planeswalker])
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let restriction = ManaUsageRestriction::CastSpellMatching {
        filter: ObjectFilter::default().with_subtype(Subtype::Dragon),
        restrict_to_matching_spell: true,
        grant_uncounterable: false,
        enters_with_counters: vec![],
        granted_abilities: vec![],
    };
    let entry = StackEntry::ability(
        source_id,
        alice,
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::add_mana_of_any_color_restricted(
                crate::effect::Value::Fixed(2),
                crate::color::Color::ALL.to_vec(),
            ),
        ]),
    )
    .with_mana_usage_restrictions(vec![restriction], None);
    game.push_to_stack(entry);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm)
        .expect("stacked loyalty-style mana ability should resolve");

    let restricted_units = game
        .player(alice)
        .expect("player should exist")
        .restricted_mana
        .clone();
    assert_eq!(restricted_units.len(), 2);
    let produced_symbol = restricted_units[0].symbol;

    let dragon_spell = CardDefinitionBuilder::new(CardId::new(), "Dragon Spell")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .build();
    let dragon_spell_id = game.create_object_from_definition(&dragon_spell, alice, Zone::Stack);
    let one_mana = ManaCost::from_symbols(vec![produced_symbol]);
    assert!(
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(dragon_spell_id),
            &one_mana,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "restricted mana produced by the stacked ability should pay for Dragon spells"
    );

    let elf_spell = CardDefinitionBuilder::new(CardId::new(), "Elf Spell")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();
    let elf_spell_id = game.create_object_from_definition(&elf_spell, alice, Zone::Stack);
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(elf_spell_id),
            &one_mana,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "restricted mana produced by the stacked ability should reject non-Dragon spells"
    );
}

#[test]
fn arena_style_exert_mana_grants_haste_through_cast_flow() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let arena = arena_style_land_definition();
    let arena_id = game.create_object_from_definition(&arena, alice, Zone::Battlefield);
    let arena_ability_index = restricted_mana_ability_index(&game, arena_id);

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut decision_maker = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateManaAbility {
            source: arena_id,
            ability_index: arena_ability_index,
        }),
        &mut decision_maker,
    )
    .expect("Arena-style mana ability should activate");
    let crate::decisions::context::DecisionContext::ManaPayment(payment) = (match progress {
        GameProgress::NeedsDecisionCtx(context) => context,
        other => panic!("Arena-style mana ability should request payment, got {other:?}"),
    }) else {
        panic!("Arena-style mana ability should request an authoritative mana payment");
    };
    apply_mana_payment_plan_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        },
        &mut decision_maker,
    )
    .expect("Arena-style mana payment should commit");

    let restricted_red = game
        .player(alice)
        .expect("alice should exist")
        .restricted_mana
        .iter()
        .filter(|unit| unit.symbol == ManaSymbol::Red)
        .count();
    assert_eq!(
        restricted_red, 2,
        "Arena-style ability should produce two restricted red mana"
    );

    let creature = CardDefinitionBuilder::new(CardId::new(), "Arena-Funded Warrior")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .build();
    let creature_id = game.create_object_from_definition(&creature, alice, Zone::Hand);

    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: creature_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut decision_maker,
    )
    .expect("creature spell should be cast with Arena mana");
    let crate::decisions::context::DecisionContext::ManaPayment(payment) = (match progress {
        GameProgress::NeedsDecisionCtx(context) => context,
        other => panic!("creature spell should request payment, got {other:?}"),
    }) else {
        panic!("creature spell should request an authoritative mana payment");
    };
    assert_eq!(
        game.object(payment.request.source)
            .expect("creature spell proposal should exist")
            .zone,
        Zone::Stack
    );
    apply_mana_payment_plan_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: payment.plan.id,
            request_hash: payment.plan.request_hash,
        },
        &mut decision_maker,
    )
    .expect("creature spell mana payment should commit");

    let stack_creature_id = game
        .stack
        .last()
        .expect("creature spell should be on the stack")
        .object_id;
    assert!(
        game.current_has_static_ability_id(stack_creature_id, StaticAbilityId::Haste),
        "creature spell should gain haste while on the stack from Arena mana"
    );

    resolve_stack_entry(&mut game).expect("creature spell should resolve");
    let permanent_id = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Arena-Funded Warrior")
        })
        .expect("creature should resolve to the battlefield");
    assert!(
        game.current_has_static_ability_id(permanent_id, StaticAbilityId::Haste),
        "creature permanent should keep haste after resolving"
    );
}

#[test]
fn test_mana_ability_undo_safe_for_basic_tap_sources() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mountain_id =
        game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);
    assert!(
        mana_ability_is_undo_safe(&game, mountain_id, 0),
        "basic tap-for-mana land should be undo-safe"
    );

    let command_tower_id =
        game.create_object_from_definition(&command_tower(), alice, Zone::Battlefield);
    assert!(
        mana_ability_is_undo_safe(&game, command_tower_id, 0),
        "tap-for-any-color mana ability should be undo-safe"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
fn test_mana_ability_undo_not_safe_for_stateful_activations() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let wall_id = game.create_object_from_definition(&wall_of_roots(), alice, Zone::Battlefield);
    let wall_mana_index = game
        .object(wall_id)
        .and_then(|obj| {
            obj.abilities
                .iter()
                .position(|ability| ability.is_mana_ability())
        })
        .expect("wall of roots should have a mana ability");
    assert!(
        !mana_ability_is_undo_safe(&game, wall_id, wall_mana_index),
        "Wall of Roots-style counter costs should not be undo-safe"
    );

    let blood_celebrant_id =
        game.create_object_from_definition(&blood_celebrant(), alice, Zone::Battlefield);
    let blood_celebrant_mana_index = game
        .object(blood_celebrant_id)
        .and_then(|obj| {
            obj.abilities
                .iter()
                .position(|ability| ability.is_mana_ability())
        })
        .expect("blood celebrant should have a mana ability");
    assert!(
        !mana_ability_is_undo_safe(&game, blood_celebrant_id, blood_celebrant_mana_index),
        "mana abilities with non-mana side effects should not be undo-safe"
    );

    let treasure_id =
        game.create_object_from_definition(&treasure_token_definition(), alice, Zone::Battlefield);
    assert!(
        !mana_ability_is_undo_safe(&game, treasure_id, 0),
        "tap+sacrifice mana abilities should not be undo-safe"
    );
}

#[test]
fn test_phyrexian_tower_alternative_mana_abilities_are_one_payment_source() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    game.create_object_from_definition(&phyrexian_tower(), alice, Zone::Battlefield);
    game.create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

    let spell = CardDefinitionBuilder::new(CardId::new(), "Tower Overcount Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Colorless],
        ]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

    let next_object_id_before_actions = game.next_object_id_counter();
    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert_eq!(
        game.next_object_id_counter(),
        next_object_id_before_actions,
        "hypothetical mana simulations must not burn committed object ids"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id: id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *id == spell_id
        )),
        "Phyrexian Tower can activate either its {{C}} ability or its sacrifice-for-{{B}}{{B}} ability, not both"
    );

    let followup = CardDefinitionBuilder::new(CardId::new(), "Post-Hypothetical Probe")
        .card_types(vec![CardType::Artifact])
        .build();
    assert_eq!(
        game.create_object_from_definition(&followup, alice, Zone::Hand),
        ObjectId::from_raw(next_object_id_before_actions),
        "the next real object should receive the first id after the pre-probe state"
    );
}

#[test]
fn mandatory_trigger_action_loop_104_4b_ends_the_game_in_a_draw() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Mandatory Life Loop")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::you_gain_life(),
            vec![crate::effect::Effect::gain_life(1)],
        ))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        crate::triggers::TriggerEvent::new(
            crate::events::LifeGainEvent::new(alice, 1),
            crate::provenance::ProvNodeId::default(),
        ),
        false,
    );
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("mandatory loop should begin");

    for _ in 0..64 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_context),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap_or_else(|error| panic!("passing through mandatory loop failed: {error}")),
            GameProgress::StackResolved => {
                advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
                    .expect("mandatory loop should continue after resolution")
            }
            GameProgress::GameOver(GameResult::Draw) => return,
            GameProgress::GameOver(other) => {
                panic!("mandatory loop produced the wrong game result: {other:?}")
            }
            GameProgress::Continue => panic!("mandatory loop incorrectly ended the phase"),
            GameProgress::NeedsDecisionCtx(other) => {
                panic!("mandatory loop unexpectedly requested a choice: {other:?}")
            }
        };
    }

    panic!("mandatory loop failed to produce the CR 104.4b draw");
}

#[test]
fn optional_priority_action_prevents_104_4b_automatic_draw() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = CardDefinitionBuilder::new(CardId::new(), "Interruptible Life Loop")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::you_gain_life(),
            vec![crate::effect::Effect::gain_life(1)],
        ))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let optional_instant = CardDefinitionBuilder::new(CardId::new(), "Optional Interruption")
        .mana_cost(ManaCost::new())
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&optional_instant, bob, Zone::Hand);

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        crate::triggers::TriggerEvent::new(
            crate::events::LifeGainEvent::new(alice, 1),
            crate::provenance::ProvNodeId::default(),
        ),
        false,
    );
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("interruptible loop should begin");
    let mut saw_optional_action = false;
    let mut resolutions = 0;

    for _ in 0..64 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(context),
            ) => {
                saw_optional_action |= context
                    .actions
                    .iter()
                    .any(|action| !matches!(action, LegalAction::PassPriority));
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                    &mut dm,
                )
                .unwrap_or_else(|error| panic!("passing through optional loop failed: {error}"))
            }
            GameProgress::StackResolved => {
                resolutions += 1;
                if resolutions == 4 {
                    break;
                }
                advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
                    .expect("optional loop should continue after resolution")
            }
            GameProgress::GameOver(GameResult::Draw) => {
                panic!("a loop with an available optional action must not draw")
            }
            GameProgress::GameOver(other) => {
                panic!("interruptible loop produced an unexpected result: {other:?}")
            }
            GameProgress::Continue => panic!("interruptible loop incorrectly ended the phase"),
            GameProgress::NeedsDecisionCtx(other) => {
                panic!("interruptible loop unexpectedly requested a choice: {other:?}")
            }
        };
    }

    assert!(saw_optional_action);
    assert_eq!(resolutions, 4);
}

#[derive(Debug, Clone)]
struct GainLifeWhileChargeCounterRemains;

impl crate::effects::EffectExecutor for GainLifeWhileChargeCounterRemains {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<crate::effect::EffectOutcome, crate::effects::ExecutionError> {
        let _ = game.remove_counters(
            ctx.source,
            crate::object::CounterType::Charge,
            1,
            Some(ctx.source),
            Some(ctx.controller),
        );
        let remaining = game
            .object(ctx.source)
            .and_then(|source| {
                source
                    .counters
                    .get(&crate::object::CounterType::Charge)
                    .copied()
            })
            .unwrap_or(0);
        if remaining > 0 {
            crate::effects::EffectExecutor::execute(
                &crate::effects::GainLifeEffect::you(1),
                game,
                ctx,
            )
        } else {
            Ok(crate::effect::EffectOutcome::resolved())
        }
    }
}

#[test]
fn finite_state_changing_trigger_chain_does_not_draw_under_104_4b() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Finite Life Chain")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::you_gain_life(),
            vec![crate::effect::Effect::new(
                GainLifeWhileChargeCounterRemains,
            )],
        ))
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    game.add_counters(source_id, crate::object::CounterType::Charge, 3)
        .expect("finite-chain source should accept charge counters");

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        crate::triggers::TriggerEvent::new(
            crate::events::LifeGainEvent::new(alice, 1),
            crate::provenance::ProvNodeId::default(),
        ),
        false,
    );
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("finite trigger chain should begin");
    let mut resolutions = 0;

    for _ in 0..64 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap_or_else(|error| panic!("passing through finite chain failed: {error}")),
            GameProgress::StackResolved => {
                resolutions += 1;
                if resolutions == 3 {
                    break;
                }
                advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
                    .expect("finite trigger chain should continue while a counter remains")
            }
            GameProgress::GameOver(GameResult::Draw) => {
                panic!("a bounded state-changing trigger chain must not draw")
            }
            GameProgress::GameOver(other) => {
                panic!("finite trigger chain produced an unexpected result: {other:?}")
            }
            GameProgress::Continue => panic!("finite trigger chain ended its phase too early"),
            GameProgress::NeedsDecisionCtx(other) => {
                panic!("finite trigger chain unexpectedly requested a choice: {other:?}")
            }
        };
    }

    assert_eq!(resolutions, 3);
    assert_eq!(
        game.object(source_id)
            .and_then(|source| source.counters.get(&crate::object::CounterType::Charge))
            .copied()
            .unwrap_or(0),
        0
    );
}

#[derive(Default)]
struct AlwaysAcceptMayDecisionMaker {
    choices: usize,
}

impl DecisionMaker for AlwaysAcceptMayDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.choices += 1;
        true
    }
}

#[test]
fn nested_may_choice_prevents_104_4b_automatic_draw() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Optional Nested Life Loop")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::you_gain_life(),
            vec![crate::effect::Effect::new(crate::effects::MayEffect::new(
                vec![crate::effect::Effect::gain_life(1)],
            ))],
        ))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        crate::triggers::TriggerEvent::new(
            crate::events::LifeGainEvent::new(alice, 1),
            crate::provenance::ProvNodeId::default(),
        ),
        false,
    );
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AlwaysAcceptMayDecisionMaker::default();
    let mut progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("optional nested loop should begin");
    let mut resolutions = 0;

    for _ in 0..64 {
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap_or_else(|error| panic!("passing through nested may loop failed: {error}")),
            GameProgress::StackResolved => {
                resolutions += 1;
                if resolutions == 4 {
                    break;
                }
                advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
                    .expect("accepted nested may loop should continue")
            }
            GameProgress::GameOver(GameResult::Draw) => {
                panic!("a loop containing a nested may choice must not draw")
            }
            GameProgress::GameOver(other) => {
                panic!("nested may loop produced an unexpected result: {other:?}")
            }
            GameProgress::Continue => panic!("nested may loop incorrectly ended the phase"),
            GameProgress::NeedsDecisionCtx(other) => {
                panic!("nested may loop unexpectedly surfaced a choice: {other:?}")
            }
        };
    }

    assert_eq!(resolutions, 4);
    assert_eq!(dm.choices, 4);
}

#[test]
fn immediate_triggered_mana_procedure_loop_104_4b_is_a_draw() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Mandatory Mana Loop")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::mana_added(crate::target::PlayerFilter::You),
            vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Green])],
        ))
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(
        &mut game,
        &mut trigger_queue,
        crate::triggers::TriggerEvent::new(
            crate::events::ManaAddedEvent::new(source_id, alice, alice, vec![ManaSymbol::Green]),
            crate::provenance::ProvNodeId::default(),
        ),
        false,
    );
    let mut dm = crate::decision::AutoPassDecisionMaker;

    assert_eq!(
        put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm),
        Err(GameLoopError::MandatoryLoopDraw)
    );
}

#[test]
fn u078_transaction_predicates_cover_cumulative_upkeep_and_costs_containing_x() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    let cumulative = RestrictedManaUnit {
        symbol: ManaSymbol::Blue,
        source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Purpose(
                ManaPaymentPurpose::CumulativeUpkeep,
            )),
            on_spend: Vec::new(),
        }],
    };
    game.player_mut(alice)
        .expect("alice")
        .add_restricted_mana(cumulative);
    let blue = ManaCost::from_symbols(vec![ManaSymbol::Blue]);

    assert!(!game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &blue,
        0,
        crate::costs::PaymentReason::Effect,
    ));
    assert!(game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &blue,
        0,
        crate::costs::PaymentReason::CumulativeUpkeep,
    ));

    let mut game = setup_game();
    let contains_x = RestrictedManaUnit {
        symbol: ManaSymbol::Colorless,
        source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::CostContainsX),
            on_spend: Vec::new(),
        }],
    };
    game.player_mut(alice)
        .expect("alice")
        .add_restricted_mana(contains_x);
    assert!(!game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
        0,
        crate::costs::PaymentReason::Other,
    ));
    assert!(game.can_pay_mana_cost_with_reason(
        alice,
        None,
        &ManaCost::from_symbols(vec![ManaSymbol::X]),
        1,
        crate::costs::PaymentReason::Other,
    ));
}

#[test]
fn typed_mana_spend_predicates_preserve_negative_cast_and_source_activation_semantics() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mana_source_definition = CardDefinitionBuilder::new(CardId::new(), "Restricted Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let mana_source =
        game.create_object_from_definition(&mana_source_definition, alice, Zone::Battlefield);

    let artifact_definition = CardDefinitionBuilder::new(CardId::new(), "Artifact Payment")
        .card_types(vec![CardType::Artifact])
        .build();
    let creature_definition = CardDefinitionBuilder::new(CardId::new(), "Creature Payment")
        .card_types(vec![CardType::Creature])
        .build();
    let artifact_spell =
        game.create_object_from_definition(&artifact_definition, alice, Zone::Stack);
    let creature_spell =
        game.create_object_from_definition(&creature_definition, alice, Zone::Stack);
    let artifact_permanent =
        game.create_object_from_definition(&artifact_definition, alice, Zone::Battlefield);
    let creature_permanent =
        game.create_object_from_definition(&creature_definition, alice, Zone::Battlefield);

    let nonartifact_cast_forbidden = RestrictedManaUnit {
        symbol: ManaSymbol::Blue,
        source: mana_source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Not(Box::new(
                ManaPaymentPredicate::All(vec![
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                    ManaPaymentPredicate::SourceMatches(
                        crate::target::ObjectFilter::default().without_type(CardType::Artifact),
                    ),
                ]),
            ))),
            on_spend: Vec::new(),
        }],
    };
    assert!(game.restricted_mana_unit_is_payable_for_reason(
        &nonartifact_cast_forbidden,
        Some(artifact_spell),
        crate::costs::PaymentReason::CastSpell,
    ));
    assert!(!game.restricted_mana_unit_is_payable_for_reason(
        &nonartifact_cast_forbidden,
        Some(creature_spell),
        crate::costs::PaymentReason::CastSpell,
    ));
    assert!(
        game.restricted_mana_unit_is_payable_for_reason(
            &nonartifact_cast_forbidden,
            Some(creature_permanent),
            crate::costs::PaymentReason::ActivateAbility,
        ),
        "a negative nonartifact-spell restriction must not forbid non-cast payments"
    );

    let hand_card = game.create_object_from_definition(&creature_definition, alice, Zone::Hand);
    let hand_origin =
        ObjectSnapshot::from_object(game.object(hand_card).expect("hand card"), &game);
    let hand_spell = game
        .move_object_by_effect(hand_card, Zone::Stack)
        .expect("hand card should move to the stack");
    game.set_cast_origin_snapshot(hand_spell, hand_origin);

    let graveyard_card =
        game.create_object_from_definition(&creature_definition, alice, Zone::Graveyard);
    let graveyard_origin =
        ObjectSnapshot::from_object(game.object(graveyard_card).expect("graveyard card"), &game);
    let graveyard_spell = game
        .move_object_by_effect(graveyard_card, Zone::Stack)
        .expect("graveyard card should move to the stack");
    game.set_cast_origin_snapshot(graveyard_spell, graveyard_origin);

    let hand_cast_forbidden = RestrictedManaUnit {
        symbol: ManaSymbol::Colorless,
        source: mana_source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Not(Box::new(
                ManaPaymentPredicate::All(vec![
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                    ManaPaymentPredicate::SourceMatches(
                        crate::target::ObjectFilter::default()
                            .in_zone(Zone::Hand)
                            .owned_by(crate::target::PlayerFilter::You),
                    ),
                ]),
            ))),
            on_spend: Vec::new(),
        }],
    };
    assert!(!game.restricted_mana_unit_is_payable_for_reason(
        &hand_cast_forbidden,
        Some(hand_spell),
        crate::costs::PaymentReason::CastSpell,
    ));
    assert!(game.restricted_mana_unit_is_payable_for_reason(
        &hand_cast_forbidden,
        Some(graveyard_spell),
        crate::costs::PaymentReason::CastSpell,
    ));
    assert!(game.restricted_mana_unit_is_payable_for_reason(
        &hand_cast_forbidden,
        Some(creature_permanent),
        crate::costs::PaymentReason::ActivateAbility,
    ));

    let artifact_source_activations_only = RestrictedManaUnit {
        symbol: ManaSymbol::Blue,
        source: mana_source,
        source_chosen_creature_type: None,
        restrictions: vec![ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::All(vec![
                ManaPaymentPredicate::AnyOf(vec![
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateAbility),
                    ManaPaymentPredicate::Purpose(ManaPaymentPurpose::ActivateManaAbility),
                ]),
                ManaPaymentPredicate::SourceMatches(
                    crate::target::ObjectFilter::default().with_type(CardType::Artifact),
                ),
            ])),
            on_spend: Vec::new(),
        }],
    };
    assert!(game.restricted_mana_unit_is_payable_for_reason(
        &artifact_source_activations_only,
        Some(artifact_permanent),
        crate::costs::PaymentReason::ActivateAbility,
    ));
    assert!(game.restricted_mana_unit_is_payable_for_reason(
        &artifact_source_activations_only,
        Some(artifact_permanent),
        crate::costs::PaymentReason::ActivateManaAbility,
    ));
    assert!(!game.restricted_mana_unit_is_payable_for_reason(
        &artifact_source_activations_only,
        Some(creature_permanent),
        crate::costs::PaymentReason::ActivateAbility,
    ));
    assert!(!game.restricted_mana_unit_is_payable_for_reason(
        &artifact_source_activations_only,
        Some(artifact_spell),
        crate::costs::PaymentReason::CastSpell,
    ));
}

#[test]
fn u078_each_doubled_mana_unit_publishes_an_event_and_queues_its_own_payload() {
    use crate::effects::{DoubleManaPoolEffect, EffectExecutor, ExecutionContext};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let mana_source_definition = CardDefinitionBuilder::new(CardId::new(), "Payload Mana Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    let mana_source =
        game.create_object_from_definition(&mana_source_definition, alice, Zone::Battlefield);
    let payload = ManaSpendPayload {
        predicate: ManaPaymentPredicate::All(vec![
            ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
            ManaPaymentPredicate::SourceMatches(
                crate::target::ObjectFilter::default().with_type(CardType::Creature),
            ),
        ]),
        effects: ResolutionProgram::from_effects(vec![crate::effect::Effect::scry(1)]),
        choices: Vec::new(),
    };
    game.player_mut(alice)
        .expect("alice")
        .add_restricted_mana(RestrictedManaUnit {
            symbol: ManaSymbol::Green,
            source: mana_source,
            source_chosen_creature_type: None,
            restrictions: vec![ManaUsageRestriction::PaymentTransaction {
                restriction: None,
                on_spend: vec![payload],
            }],
        });

    let mut ctx = ExecutionContext::new_default(mana_source, alice);
    DoubleManaPoolEffect::you()
        .execute(&mut game, &mut ctx)
        .expect("doubling should preserve the complete mana-unit payload");

    let creature = CardDefinitionBuilder::new(CardId::new(), "Paid Creature")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .card_types(vec![CardType::Creature])
        .build();
    let spell = game.create_object_from_definition(&creature, alice, Zone::Stack);
    assert!(game.try_pay_mana_cost_with_reason(
        alice,
        Some(spell),
        &ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]),
        0,
        crate::costs::PaymentReason::CastSpell,
    ));

    let spent_events = game
        .take_pending_trigger_events()
        .into_iter()
        .filter(|event| {
            event
                .downcast::<crate::events::ManaUnitSpentEvent>()
                .is_some()
        })
        .count();
    assert_eq!(
        spent_events, 2,
        "one spend record is required per concrete unit"
    );

    let entries = game.take_pending_trigger_entries();
    assert_eq!(entries.len(), 2, "CR 106.6a requires one trigger per unit");
    assert!(entries.iter().all(|entry| {
        entry
            .tagged_objects
            .get(ironsmith_core::MANA_PAID_OBJECT_TAG)
            .is_some_and(|snapshots| snapshots.len() == 1 && snapshots[0].object_id == spell)
            && entry.ability.effects.all_effects().iter().any(|effect| {
                effect
                    .downcast_ref::<crate::effects::ScryEffect>()
                    .is_some()
            })
    }));
}

#[test]
fn u078_on_spend_predicate_does_not_restrict_ordinary_use_or_trigger_on_mismatch() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    game.player_mut(alice)
        .expect("alice")
        .add_restricted_mana(RestrictedManaUnit {
            symbol: ManaSymbol::Red,
            source,
            source_chosen_creature_type: None,
            restrictions: vec![ManaUsageRestriction::PaymentTransaction {
                restriction: None,
                on_spend: vec![ManaSpendPayload {
                    predicate: ManaPaymentPredicate::SourceMatches(
                        crate::target::ObjectFilter::default().with_type(CardType::Creature),
                    ),
                    effects: ResolutionProgram::from_effects(vec![crate::effect::Effect::scry(1)]),
                    choices: Vec::new(),
                }],
            }],
        });
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Paid Artifact")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .card_types(vec![CardType::Artifact])
        .build();
    let spell = game.create_object_from_definition(&artifact, alice, Zone::Stack);

    assert!(game.try_pay_mana_cost_with_reason(
        alice,
        Some(spell),
        &ManaCost::from_symbols(vec![ManaSymbol::Red]),
        0,
        crate::costs::PaymentReason::CastSpell,
    ));
    assert!(game.take_pending_trigger_entries().is_empty());
}

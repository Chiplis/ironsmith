//! CR audit regressions: assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.
//! All cases remain in this integration-test crate.

use ironsmith::decisions::context::BooleanContext;
use ironsmith::effects::RedistributeLifeTotalsEffect;
use ironsmith::{
    DecisionMaker, EffectContext, EffectExecutor, GameState, PlayerFilter, PlayerId,
    SelectOptionsContext,
};

fn counter_creature(game: &mut GameState, zone: ironsmith::Zone) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    let definition = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Counter recipient")
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    game.create_object_from_definition(&definition, PlayerId::from_index(0), zone)
}

struct InspectDamageSourceChoice {
    candidate: ironsmith::ObjectId,
    expected_legal: bool,
    seen: usize,
}

#[test]
fn cr_120_6_marked_damage_survives_type_loss_until_cleanup() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect};
    use ironsmith::{CardType, ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let recipient = counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.damage_on(recipient), 3);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(recipient),
        Modification::SetCardTypes(vec![CardType::Artifact]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(!game.current_is_creature(recipient));
    ironsmith::rules::state_based::apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        game.damage_on(recipient),
        3,
        "CR 120.6: type loss retains damage"
    );
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(game.current_is_creature(recipient));
    assert_eq!(
        game.damage_on(recipient),
        0,
        "CR 120.6 / 514.2: cleanup removes damage"
    );
}

#[test]
fn cr_120_6_regeneration_removes_all_marked_damage() {
    use ironsmith::effects::{DealDamageEffect, DestroyEffect, RegenerateEffect};
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let recipient = counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    RegenerateEffect::new(
        ChooseSpec::SpecificObject(recipient),
        ironsmith::Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(
        game.damage_on(recipient),
        3,
        "Creating a shield is not regeneration"
    );
    DestroyEffect::with_spec(ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.object(recipient).unwrap().zone, Zone::Battlefield);
    assert!(game.is_tapped(recipient));
    assert_eq!(
        game.damage_on(recipient),
        0,
        "CR 120.6: regeneration removes damage"
    );
}
impl DecisionMaker for InspectDamageSourceChoice {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        self.seen += 1;
        assert_eq!(
            ctx.candidates
                .iter()
                .any(|c| c.id == self.candidate && c.legal),
            self.expected_legal,
            "CR 120.7 / 609.7a / 702.26b: damage source choice has incorrect candidate membership"
        );
        vec![ctx.candidates.iter().find(|c| c.legal).unwrap().id]
    }
}

fn audit_damage_source_choice(case: u8) {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::PreventAllDamageToTargetEffect;
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::{CardId, CardType, ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let zone = match case {
        0 => Zone::Command,
        1 => Zone::Graveyard,
        4 => Zone::Stack,
        5 => Zone::Hand,
        _ => Zone::Battlefield,
    };
    let definition = CardDefinitionBuilder::new(CardId::new(), "Potential damage source")
        .card_types(vec![CardType::Artifact])
        .build();
    let candidate = game.create_object_from_definition(&definition, alice, zone);
    if case == 1 {
        let spell =
            CardDefinitionBuilder::new(CardId::new(), "Spell referring to a graveyard card")
                .card_types(vec![CardType::Instant])
                .build();
        let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
        game.stack
            .push(StackEntry::new(spell, alice).with_targets(vec![Target::Object(candidate)]));
    }
    if case == 2 {
        game.phase_out(candidate);
        assert!(game.is_phased_out(candidate));
    }
    if case == 4 {
        game.stack.push(StackEntry::new(candidate, alice));
    }
    let mut choices = InspectDamageSourceChoice {
        candidate,
        expected_legal: !matches!(case, 2 | 5),
        seen: 0,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut choices);
    let mut prevention = PreventAllDamageToTargetEffect::new(
        ChooseSpec::Player(PlayerFilter::You),
        Until::EndOfTurn,
    );
    prevention.source_of_your_choice = true;
    prevention.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(choices.seen, 1);
}

#[test]
fn cr_120_7_face_up_command_zone_object_is_a_legal_damage_source_choice() {
    audit_damage_source_choice(0);
}

#[test]
fn cr_120_7_object_referred_to_by_a_stack_spell_is_a_legal_damage_source_choice() {
    audit_damage_source_choice(1);
}

#[test]
fn cr_702_26b_phased_out_permanent_is_not_a_damage_source_choice() {
    audit_damage_source_choice(2);
}

#[test]
fn cr_120_7_source_choices_include_nondamaging_permanents_and_permanent_spells_but_not_arbitrary_hand_cards()
 {
    for case in [3, 4, 5] {
        audit_damage_source_choice(case);
    }
}

#[test]
fn cr_306_5a_loyalty_counters_do_not_change_printed_loyalty_outside_battlefield() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, CounterType, Zone};
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Off-battlefield loyalty")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(4)
            .build();
        let card = game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
        assert_eq!(game.object(card).unwrap().loyalty(), Some(4));
        game.add_counters(card, CounterType::Loyalty, 2);
        assert_eq!(
            game.object(card).unwrap().loyalty(),
            Some(4),
            "CR 306.5a / 209.1: loyalty outside the battlefield is printed loyalty, regardless of counters; {zone:?}"
        );
    }
}

fn audit_damage_source(game: &mut GameState, flags: u8) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, PowerToughness, StaticAbility, Zone};
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Damage source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4));
    for (bit, ability) in [
        (1, StaticAbility::infect()),
        (2, StaticAbility::wither()),
        (4, StaticAbility::lifelink()),
    ] {
        if flags & bit != 0 {
            definition = definition.with_ability(Ability::static_ability(ability));
        }
    }
    game.create_object_from_definition(
        &definition.build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    )
}

#[test]
fn cr_120_3_permanent_damage_applies_every_relevant_type_and_source_keyword_result() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, PowerToughness, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for flags in 0..8 {
        for types in 1..8 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = audit_damage_source(&mut game, flags);
            let card_types = [
                (1, CardType::Creature),
                (2, CardType::Planeswalker),
                (4, CardType::Battle),
            ]
            .into_iter()
            .filter(|(bit, _)| types & bit != 0)
            .map(|(_, kind)| kind)
            .collect();
            let definition = CardDefinitionBuilder::new(CardId::new(), "Damage recipient")
                .card_types(card_types)
                .power_toughness(PowerToughness::fixed(3, 4))
                .loyalty(5)
                .defense(6)
                .build();
            let target = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let loyalty_before = game.counter_count(target, CounterType::Loyalty);
            let defense_before = game.counter_count(target, CounterType::Defense);
            let mut ctx = EffectContext::new_default(source, alice);
            DealDamageEffect::new(2, ChooseSpec::SpecificObject(target))
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(
                game.damage_on(target),
                if types & 1 != 0 && flags & 3 == 0 {
                    2
                } else {
                    0
                },
                "marked damage: source flags={flags}, recipient types={types}"
            );
            assert_eq!(
                game.counter_count(target, CounterType::MinusOneMinusOne),
                if types & 1 != 0 && flags & 3 != 0 {
                    2
                } else {
                    0
                },
                "wither/infect counters: flags={flags}, types={types}"
            );
            assert_eq!(
                game.counter_count(target, CounterType::Loyalty),
                loyalty_before - if types & 2 != 0 { 2 } else { 0 }
            );
            assert_eq!(
                game.counter_count(target, CounterType::Defense),
                defense_before - if types & 4 != 0 { 2 } else { 0 }
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                if flags & 4 != 0 { 22 } else { 20 }
            );
        }
    }
}

#[test]
fn cr_120_3_player_damage_distinguishes_infect_wither_and_lifelink() {
    use ironsmith::ChooseSpec;
    use ironsmith::effects::DealDamageEffect;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for flags in 0..8 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = audit_damage_source(&mut game, flags);
        let mut ctx = EffectContext::new_default(source, alice);
        DealDamageEffect::new(2, ChooseSpec::Player(PlayerFilter::Specific(bob)))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.player(bob).unwrap().life,
            if flags & 1 != 0 { 20 } else { 18 }
        );
        assert_eq!(
            game.player(bob).unwrap().poison_counters,
            if flags & 1 != 0 { 2 } else { 0 }
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if flags & 4 != 0 { 22 } else { 20 }
        );
    }
}

fn audit_damage_counter_actor_and_lifelink(player_recipient: bool) {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::events::MarkersChangedEvent;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, CounterType, PowerToughness, StaticAbility, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Transferred infect source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::infect()))
        .with_ability(Ability::static_ability(StaticAbility::lifelink()))
        .build();
    let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    let victim = counter_creature(&mut game, Zone::Battlefield);
    // Alice controls the resolving ability; Bob now controls its source.
    let mut ctx = EffectContext::new_default(source, alice);
    let target = if player_recipient {
        ChooseSpec::Player(PlayerFilter::Specific(charlie))
    } else {
        ChooseSpec::SpecificObject(victim)
    };
    let outcome = DealDamageEffect::new(2, target)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.player(charlie).unwrap().poison_counters,
        if player_recipient { 2 } else { 0 }
    );
    assert_eq!(
        game.counter_count(victim, CounterType::MinusOneMinusOne),
        if player_recipient { 0 } else { 2 }
    );
    assert_eq!(game.player(bob).unwrap().life, 22);
    assert_eq!(game.player(alice).unwrap().life, 20);
    let placement = outcome
        .events
        .iter()
        .find_map(|e| e.downcast::<MarkersChangedEvent>())
        .expect("damage counter placement event");
    assert_eq!(
        placement.source_controller,
        Some(bob),
        "CR 120.3b/d: the damage source's controller places counters, not the ability controller"
    );
}

#[test]
fn cr_120_3_damage_counter_actor_and_lifelink_use_current_source_controller() {
    audit_damage_counter_actor_and_lifelink(true);
}
#[test]
fn cr_120_3d_creature_counter_actor_uses_current_source_controller() {
    audit_damage_counter_actor_and_lifelink(false);
}

#[test]
fn cr_120_8_zero_damage_does_not_apply_a_damage_increase_or_emit_damage() {
    use ironsmith::ChooseSpec;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::events::damage::matchers::DamageToPlayerMatcher;
    use ironsmith::events::{DamageEvent, LifeGainEvent};
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for amount in [0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = audit_damage_source(&mut game, 4);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(3)),
            ),
        );
        let mut ctx = EffectContext::new_default(source, alice);
        let outcome =
            DealDamageEffect::new(amount, ChooseSpec::Player(PlayerFilter::Specific(bob)))
                .execute(&mut game, &mut ctx)
                .unwrap();
        assert_eq!(
            game.player(bob).unwrap().life,
            if amount == 0 { 20 } else { 16 }
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if amount == 0 { 20 } else { 24 }
        );
        if amount == 0 {
            assert!(
                !outcome
                    .events
                    .iter()
                    .any(|e| e.downcast::<DamageEvent>().is_some())
            );
            assert!(
                !outcome
                    .events
                    .iter()
                    .any(|e| e.downcast::<LifeGainEvent>().is_some())
            );
        }
    }
}

#[test]
fn cr_120_5_lethal_damage_leaves_a_creature_until_state_based_actions() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = audit_damage_source(&mut game, 0);
    let target = counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    DealDamageEffect::new(5, ChooseSpec::SpecificObject(target))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(
        game.battlefield.contains(&target),
        "damage itself does not destroy the creature"
    );
    assert_eq!(game.damage_on(target), 5);
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(!game.battlefield.contains(&target));
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
}

#[test]
fn cr_122_1e_zero_loyalty_does_not_revert_to_printed_starting_loyalty() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Loyalty test planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let walker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert_eq!(game.object(walker).unwrap().loyalty(), Some(4));
    game.remove_counters(walker, CounterType::Loyalty, 4, None, None)
        .unwrap();
    assert_eq!(game.counter_count(walker, CounterType::Loyalty), 0);
    assert_eq!(
        game.object(walker).unwrap().loyalty(),
        Some(0),
        "CR 122.1e: current loyalty is the number of loyalty counters even before the next SBA"
    );
}

#[test]
fn cr_120_4a_zero_loyalty_creature_planeswalker_treats_all_damage_as_excess() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, PowerToughness, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Animated planeswalker")
        .card_types(vec![CardType::Creature, CardType::Planeswalker])
        .power_toughness(PowerToughness::fixed(3, 4))
        .loyalty(4)
        .build();
    let walker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    // This is the state between two instructions in a resolving ability:
    // remove all loyalty counters, then deal damage with excess redirected.
    game.remove_counters(walker, CounterType::Loyalty, 4, None, None)
        .unwrap();
    let mut ctx = EffectContext::new_default(walker, alice);
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(walker))
        .with_excess_to_controller(ironsmith_core::effect::ExcessDamageRedirect { condition: None })
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().life,
        17,
        "CR 120.4a: use the greatest excess among types; zero loyalty makes all three damage excess"
    );
}

#[test]
fn cr_120_10_excess_damage_trigger_uses_loyalty_immediately_before_damage() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::triggers::{IsDealtDamageTrigger, TriggerContext, TriggerMatcher};
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, Zone};
    for loyalty in [1, 0] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Excess damage recipient")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(4)
            .build();
        let walker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_counters(walker, CounterType::Loyalty, 4 - loyalty, None, None)
            .unwrap();
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), loyalty);
        // No state-based action occurs between these two instructions.
        let outcome = DealDamageEffect::new(3, ChooseSpec::SpecificObject(walker))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let event = outcome
            .events
            .iter()
            .find(|event| event.downcast::<ironsmith::events::DamageEvent>().is_some())
            .unwrap();
        let trigger = IsDealtDamageTrigger::excess_noncombat(ChooseSpec::SpecificObject(walker));
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(
            trigger.matches(event, &ctx),
            "CR 120.10: three damage exceeds {loyalty} loyalty and must match the trigger"
        );
        assert_eq!(
            trigger.event_value_amount(event, &ctx),
            Some((3 - loyalty) as i32),
            "CR 120.10: trigger exports damage minus pre-damage loyalty"
        );
    }
}

struct CheckDrawOffer {
    expected_legal: bool,
    seen: usize,
}
impl DecisionMaker for CheckDrawOffer {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        self.seen += 1;
        assert_eq!(
            ctx.can_accept, self.expected_legal,
            "CR 121.2b / 121.3: optional draw acceptance must respect draw prohibitions"
        );
        false
    }
}

fn audit_optional_draw(
    restriction: Option<ironsmith::effect::Restriction>,
    drawer: PlayerId,
    count: i32,
) {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{DrawCardsEffect, MayEffect};
    use ironsmith::{Ability, CardId, CardType, Effect, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let expected_legal = restriction.is_none();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Optional draw source")
        .card_types(vec![CardType::Enchantment]);
    if let Some(restriction) = restriction {
        definition = definition.with_ability(Ability::static_ability(StaticAbility::restriction(
            restriction,
            "Draw restriction".into(),
        )));
    }
    let source = game.create_object_from_definition(&definition.build(), alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let mut choices = CheckDrawOffer {
        expected_legal,
        seen: 0,
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut choices);
    MayEffect::new_for_player(
        vec![Effect::new(DrawCardsEffect::new(
            count,
            PlayerFilter::Specific(drawer),
        ))],
        PlayerFilter::Specific(alice),
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    if expected_legal {
        assert_eq!(
            choices.seen, 1,
            "an empty library must not suppress a legal draw offer"
        );
    }
}

#[test]
fn cr_121_3_an_empty_library_does_not_make_an_optional_draw_illegal() {
    audit_optional_draw(None, PlayerId::from_index(0), 1);
}

#[test]
fn cr_121_3_a_draw_prohibition_makes_an_optional_draw_illegal() {
    audit_optional_draw(
        Some(ironsmith::effect::Restriction::DrawCards(PlayerFilter::Any)),
        PlayerId::from_index(0),
        1,
    );
}

#[test]
fn cr_121_3a_draw_prohibition_applies_even_when_another_player_makes_the_choice() {
    let bob = PlayerId::from_index(1);
    audit_optional_draw(
        Some(ironsmith::effect::Restriction::DrawCards(
            PlayerFilter::Specific(bob),
        )),
        bob,
        1,
    );
}

#[test]
fn cr_121_2b_cannot_choose_to_draw_two_under_a_one_draw_limit() {
    audit_optional_draw(
        Some(ironsmith::effect::Restriction::DrawExtraCards(
            PlayerFilter::Any,
        )),
        PlayerId::from_index(0),
        2,
    );
}

#[test]
fn cr_121_2b_mandatory_draw_is_partial_but_a_multiple_draw_cost_is_unpayable() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Restriction;
    use ironsmith::effects::{CostExecutableEffect, DrawCardsEffect};
    use ironsmith::{Ability, CardId, CardType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    counter_creature(&mut game, Zone::Library);
    counter_creature(&mut game, Zone::Library);
    let definition = CardDefinitionBuilder::new(CardId::new(), "One draw per turn")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            Restriction::DrawExtraCards(PlayerFilter::Any),
            "One draw each turn".into(),
        )))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    let draw = DrawCardsEffect::you(2);
    assert!(CostExecutableEffect::can_execute_as_cost(&draw, &game, source, alice).is_err());
    let mut ctx = EffectContext::new_default(source, alice);
    draw.execute(&mut game, &mut ctx).unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert!(
        !game
            .player(alice)
            .unwrap()
            .attempted_draw_from_empty_library
    );
}

#[test]
fn cr_122_1g_zero_defense_siege_waits_for_any_of_its_triggered_abilities() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::events::zones::EnterBattlefieldEvent;
    use ironsmith::game_state::StackEntry;
    use ironsmith::rules::state_based::StateBasedAction;
    use ironsmith::triggers::{Trigger, TriggerEvent};
    use ironsmith::{Ability, CardId, CardType, CounterType, Effect, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for defeat_trigger in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Siege with an entry trigger")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![Subtype::Siege])
            .defense(1)
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let battle = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        // A reachable state after the defeat trigger was countered while the
        // earlier entry trigger remains on the stack.
        game.object_mut(battle)
            .unwrap()
            .counters
            .remove(&CounterType::Defense);
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(battle, Zone::Stack),
            Default::default(),
        );
        let mut entry = StackEntry::ability(battle, alice, vec![Effect::gain_life(1)])
            .with_triggering_event(event);
        if defeat_trigger {
            entry = entry.with_battle_defeat_source(battle);
        }
        game.stack.push(entry);
        assert!(
            !ironsmith::check_state_based_actions(&game)
                .contains(&StateBasedAction::BattleDies(battle)),
            "CR 122.1g / 310.7 / 704.5v: any triggered ability from this Siege delays the zero-defense SBA; defeat_trigger={defeat_trigger}"
        );
        game.stack.clear();
        assert!(
            ironsmith::check_state_based_actions(&game)
                .contains(&StateBasedAction::BattleDies(battle))
        );
    }
}

#[test]
fn cr_122_1c_shields_replace_each_effect_destruction_once() {
    use ironsmith::effects::DestroyEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(creature, CounterType::Shield, 2);
    for remaining in [1, 0] {
        let mut ctx = EffectContext::new_default(creature, alice);
        DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.counter_count(creature, CounterType::Shield), remaining);
    }
    let mut ctx = EffectContext::new_default(creature, alice);
    DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(!game.battlefield.contains(&creature));
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
}

#[test]
fn cr_122_1c_shields_prevent_damage_but_still_lose_one_counter_for_unpreventable_damage() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    for unpreventable in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = counter_creature(&mut game, Zone::Battlefield);
        game.add_counters(creature, CounterType::Shield, 2);
        let mut ctx = EffectContext::new_default(creature, PlayerId::from_index(0));
        DealDamageEffect::new(3, ChooseSpec::SpecificObject(creature))
            .with_unpreventable(unpreventable)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.counter_count(creature, CounterType::Shield), 1);
        assert_eq!(game.damage_on(creature), if unpreventable { 3 } else { 0 });
    }
}

#[test]
fn cr_122_1h_finality_replaces_only_battlefield_to_graveyard_moves() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    for (origin, destination, expected) in [
        (Zone::Battlefield, Zone::Graveyard, Zone::Exile),
        (Zone::Hand, Zone::Graveyard, Zone::Graveyard),
        (Zone::Battlefield, Zone::Hand, Zone::Hand),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = counter_creature(&mut game, origin);
        let card_id = game.object(creature).unwrap().card;
        game.add_counters(creature, CounterType::Finality, 2);
        let mut ctx = EffectContext::new_default(creature, alice);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(creature), destination, false)
            .execute(&mut game, &mut ctx)
            .unwrap();
        let ids = match expected {
            Zone::Hand => &game.player(alice).unwrap().hand,
            Zone::Graveyard => &game.player(alice).unwrap().graveyard,
            Zone::Exile => &game.exile,
            _ => unreachable!(),
        };
        assert!(
            ids.iter()
                .any(|id| game.object(*id).unwrap().card == card_id),
            "CR 122.1h: {origin:?} -> {destination:?} must end in {expected:?}"
        );
    }
}

#[test]
fn cr_122_8_and_122_9_referenced_counters_are_placed_from_departed_source_snapshot() {
    use ironsmith::effects::MoveAllCountersEffect;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let donor = counter_creature(&mut game, Zone::Battlefield);
    let recipient = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(donor, CounterType::PlusOnePlusOne, 2);
    game.add_counters(donor, CounterType::Charge, 3);
    let snapshot = ObjectSnapshot::from_object(game.object(donor).unwrap(), &game);
    let departed = game.move_object_by_effect(donor, Zone::Graveyard).unwrap();
    assert!(game.object(departed).unwrap().counters.is_empty());
    let mut ctx = EffectContext::new_default(donor, alice).with_source_snapshot(snapshot);
    MoveAllCountersEffect::put_referenced(
        ChooseSpec::Source,
        ChooseSpec::SpecificObject(recipient),
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(
        game.counter_count(recipient, CounterType::PlusOnePlusOne),
        2
    );
    assert_eq!(game.counter_count(recipient, CounterType::Charge), 3);
    assert!(game.object(departed).unwrap().counters.is_empty());
}

#[test]
fn cr_122_7_nth_counter_trigger_uses_the_counts_at_placement_time() {
    use ironsmith::triggers::{CounterPutOnTrigger, TriggerContext, TriggerMatcher};
    use ironsmith::{CounterType, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(creature, CounterType::Charge, 3);
    let crossing = game.add_counters(creature, CounterType::Charge, 2).unwrap();
    let trigger = CounterPutOnTrigger::new(ObjectFilter::creature())
        .counter_type(CounterType::Charge)
        .counter_number(4);
    assert!(trigger.matches(
        &crossing,
        &TriggerContext::for_source(creature, alice, &game)
    ));
    assert_eq!(trigger.trigger_count(&crossing), 1);
    game.add_counters(creature, CounterType::Charge, 1);
    assert!(
        trigger.matches(
            &crossing,
            &TriggerContext::for_source(creature, alice, &game)
        ),
        "CR 122.7: a later placement cannot erase the earlier event's crossing from three to five"
    );
}

#[test]
fn cr_122_4_counter_limits_are_enforced_by_sba_only_on_battlefield_permanents() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, CounterType, StaticAbility, Zone};
    for zone in [Zone::Battlefield, Zone::Hand, Zone::Graveyard] {
        for maximum in [0, 2, 5] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Limited counters")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::counter_limit_rule(
                    CounterType::Charge,
                    maximum,
                    "Charge counter limit",
                )))
                .build();
            let object =
                game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
            game.add_counters(object, CounterType::Charge, 4);
            game.add_counters(object, CounterType::Age, 3);
            assert_eq!(game.counter_count(object, CounterType::Charge), 4);
            ironsmith::apply_state_based_actions(&mut game).unwrap();
            assert_eq!(
                game.counter_count(object, CounterType::Charge),
                if zone == Zone::Battlefield {
                    maximum.min(4)
                } else {
                    4
                }
            );
            assert_eq!(game.counter_count(object, CounterType::Age), 3);
        }
    }
}

#[test]
fn cr_122_1j_hone_counters_on_equipment_increase_equipped_creatures_power() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, CounterType, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Honed equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(equipment, CounterType::Named("hone".into()), 2);
    assert_eq!(
        game.current_power(creature),
        Some(3),
        "unattached hone counters do not affect a creature"
    );
    assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(creature)));
    assert_eq!(game.current_toughness(creature), Some(4));
    assert_eq!(
        game.current_power(creature),
        Some(5),
        "CR 122.1j: each hone counter gives the equipped creature +1/+0"
    );
}

#[test]
fn cr_122_1b_hexproof_from_white_counter_blocks_white_opponent_targets() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        Ability, CardId, CardType, ColorSet, CounterType, ObjectFilter, StaticAbility, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    // The printed-ability control confirms the targeting fixture, then the
    // same protection must be supplied by the keyword-variant counter.
    for use_counter in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut definition = CardDefinitionBuilder::new(CardId::new(), "Protected creature")
            .card_types(vec![CardType::Creature]);
        if !use_counter {
            definition = definition.with_ability(Ability::static_ability(
                StaticAbility::hexproof_from(ObjectFilter::default().with_colors(ColorSet::WHITE)),
            ));
        }
        let target =
            game.create_object_from_definition(&definition.build(), alice, Zone::Battlefield);
        if use_counter {
            game.add_counters(target, CounterType::Named("hexproof from white".into()), 1);
        }
        let source_definition = CardDefinitionBuilder::new(CardId::new(), "White source")
            .card_types(vec![CardType::Instant])
            .color_indicator(ColorSet::WHITE)
            .build();
        let source = game.create_object_from_definition(&source_definition, bob, Zone::Stack);
        assert!(
            ironsmith::targeting::can_target_object(&game, target, source, alice).is_legal(),
            "hexproof does not prohibit its controller's targeting"
        );
        assert!(
            !ironsmith::targeting::can_target_object(&game, target, source, bob).is_legal(),
            "CR 122.1b includes keyword variants: hexproof from white must prohibit the opponent's white source; counter={use_counter}"
        );
    }
}

#[test]
fn cr_122_1a_stat_counters_apply_in_every_creature_card_zone() {
    use ironsmith::{CounterType, Zone};
    for zone in [
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
        Zone::Stack,
    ] {
        for (kind, power, toughness) in [
            (CounterType::PlusOnePlusOne, 1, 1),
            (CounterType::MinusOneMinusOne, -1, -1),
            (CounterType::PlusOnePlusZero, 1, 0),
            (CounterType::PlusZeroPlusOne, 0, 1),
            (CounterType::PlusOnePlusTwo, 1, 2),
            (CounterType::PlusTwoPlusTwo, 2, 2),
            (CounterType::MinusZeroMinusOne, 0, -1),
            (CounterType::MinusZeroMinusTwo, 0, -2),
            (CounterType::MinusTwoMinusOne, -2, -1),
            (CounterType::MinusTwoMinusTwo, -2, -2),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let creature = counter_creature(&mut game, zone);
            game.add_counters(creature, kind, 2);
            assert_eq!(
                game.current_power(creature),
                Some(3 + 2 * power),
                "{kind:?} in {zone:?}"
            );
            assert_eq!(
                game.current_toughness(creature),
                Some(4 + 2 * toughness),
                "{kind:?} in {zone:?}"
            );
        }
    }
}

#[test]
fn cr_122_1a_plus_two_plus_zero_counters_modify_power() {
    use ironsmith::{CounterType, Zone};
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(creature, CounterType::Named("+2/+0".into()), 2);
    assert_eq!(game.current_toughness(creature), Some(4));
    assert_eq!(
        game.current_power(creature),
        Some(7),
        "CR 122.1a applies to +X/+Y counters, including +2/+0 counters represented by a named counter"
    );
}

#[test]
fn cr_122_2_zone_change_discards_counters_without_a_counter_removal_event() {
    use ironsmith::{CounterType, Zone};
    for destination in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = counter_creature(&mut game, Zone::Battlefield);
        game.add_counters(creature, CounterType::PlusOnePlusOne, 2);
        game.add_counters(creature, CounterType::Charge, 3);
        game.take_pending_trigger_events();
        let moved = game
            .move_object_by_effect(creature, destination)
            .expect("zone move succeeds");
        assert_ne!(moved, creature, "the destination contains a new object");
        assert!(
            game.object(moved).unwrap().counters.is_empty(),
            "counters cease to exist on a zone move"
        );
        let events = game.take_pending_trigger_events();
        assert!(
            !events
                .iter()
                .any(|e| e.kind() == ironsmith::events::EventKind::MarkersChanged),
            "ceasing to exist is not counter removal"
        );
    }
}

#[test]
fn cr_122_3_opposing_counters_cancel_only_on_permanents_as_a_state_based_action() {
    use ironsmith::{CounterType, Zone};
    for zone in [Zone::Battlefield, Zone::Hand, Zone::Graveyard, Zone::Exile] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let creature = counter_creature(&mut game, zone);
        game.add_counters(creature, CounterType::PlusOnePlusOne, 3);
        game.add_counters(creature, CounterType::MinusOneMinusOne, 2);
        assert_eq!(
            game.counter_count(creature, CounterType::PlusOnePlusOne),
            3,
            "not removed until SBAs"
        );
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        let expected = if zone == Zone::Battlefield {
            (1, 0)
        } else {
            (3, 2)
        };
        assert_eq!(
            game.counter_count(creature, CounterType::PlusOnePlusOne),
            expected.0
        );
        assert_eq!(
            game.counter_count(creature, CounterType::MinusOneMinusOne),
            expected.1
        );
    }
}

#[test]
fn cr_122_5_counter_move_does_not_place_counters_when_removal_is_prohibited() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Restriction;
    use ironsmith::effects::MoveCountersEffect;
    use ironsmith::{
        Ability, CardId, CardType, CounterType, ObjectFilter, ResolvedTarget, StaticAbility, Zone,
    };
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let from = counter_creature(&mut game, Zone::Battlefield);
    let to = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(from, CounterType::PlusOnePlusOne, 3);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Counter removal prohibition")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::restriction(
            Restriction::HaveCounterTypeRemoved(
                ObjectFilter::creature(),
                CounterType::PlusOnePlusOne,
            ),
            "+1/+1 counters can't be removed from creatures".into(),
        )))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(!game.can_have_counter_type_removed(from, CounterType::PlusOnePlusOne));
    let mut ctx = EffectContext::new_default(source, alice).with_targets(vec![
        ResolvedTarget::Object(from),
        ResolvedTarget::Object(to),
    ]);
    MoveCountersEffect::plus_one_counters(2)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 3);
    assert_eq!(
        game.counter_count(to, CounterType::PlusOnePlusOne),
        0,
        "CR 122.5: when removal is impossible, no counter is put onto the recipient"
    );
}

#[test]
fn cr_122_1d_each_untap_attempt_removes_only_one_stun_counter() {
    use ironsmith::effects::UntapEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    game.tap(creature);
    game.add_counters(creature, CounterType::Stun, 2);
    for (remaining, tapped) in [(1, true), (0, true), (0, false)] {
        let mut ctx = EffectContext::new_default(creature, PlayerId::from_index(0));
        UntapEffect::target(ChooseSpec::Source)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.counter_count(creature, CounterType::Stun), remaining);
        assert_eq!(game.is_tapped(creature), tapped);
    }
}

#[test]
fn cr_121_4_empty_library_draw_is_detected_only_for_a_positive_draw_attempt() {
    use ironsmith::effects::DrawCardsEffect;
    use ironsmith::rules::state_based::{LoseReason, StateBasedAction};
    let alice = PlayerId::from_index(0);
    for count in [0, 1, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.new_object_id();
        let mut ctx = EffectContext::new_default(source, alice);
        DrawCardsEffect::you(count)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(
            game.player(alice).unwrap().is_in_game(),
            "loss waits for the SBA check"
        );
        assert_eq!(
            game.player(alice)
                .unwrap()
                .attempted_draw_from_empty_library,
            count > 0
        );
        assert_eq!(ironsmith::check_state_based_actions(&game).iter().any(|a| matches!(a,
            StateBasedAction::PlayerLoses { player, reason: LoseReason::DrewFromEmptyLibrary } if *player == alice
        )), count > 0);
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().is_in_game(), count == 0);
    }
}

#[test]
fn cr_121_2_and_121_6_replaced_draws_finish_individually_even_with_an_empty_library() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DrawCardsEffect;
    use ironsmith::events::cards::matchers::WouldDrawCardMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{CardId, CardType, Effect, Value, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Draw replacement source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![Effect::gain_life(Value::LifeTotal(
                PlayerFilter::You,
            ))]),
        ));
    let mut ctx = EffectContext::new_default(source, alice);
    DrawCardsEffect::you(3)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.player(alice).unwrap().life,
        160,
        "each replacement must finish before the next draw: 20 -> 40 -> 80 -> 160"
    );
    assert!(
        !game
            .player(alice)
            .unwrap()
            .attempted_draw_from_empty_library,
        "CR 121.6a: replacement applies even with an empty library"
    );
    assert!(game.player(alice).unwrap().hand.is_empty());
}

struct InspectRedistribution {
    forbidden_total: i32,
    rule: &'static str,
}

impl DecisionMaker for InspectRedistribution {
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        true
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let forbidden = format!("{} life", self.forbidden_total);
        assert!(
            !ctx.options
                .iter()
                .any(|o| o.legal && o.description == forbidden),
            "{}: redistribution must not offer the prohibited life total as a legal assignment; options: {:?}",
            self.rule,
            ctx.options
        );
        vec![
            ctx.options
                .iter()
                .find(|o| o.legal)
                .expect("an unchanged assignment is legal")
                .index,
        ]
    }
}

fn restricted_redistribution(cannot_gain: bool) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.player_mut(alice).unwrap().life = if cannot_gain { 10 } else { 20 };
    game.player_mut(bob).unwrap().life = if cannot_gain { 20 } else { 10 };
    // Focus the redistribution executor on an already-derived restriction.
    // It must constrain choices, not choose an illegal assignment then skip it.
    if cannot_gain {
        game.effect_store.cant_effects.cant_gain_life.insert(alice);
    } else {
        game.effect_store.cant_effects.cant_lose_life.insert(alice);
    }
    let source = game.new_object_id();
    let mut choices = InspectRedistribution {
        forbidden_total: if cannot_gain { 20 } else { 10 },
        rule: if cannot_gain { "CR 119.7" } else { "CR 119.8" },
    };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut choices);
    RedistributeLifeTotalsEffect::new(PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("redistribution with the unchanged assignment must resolve");
    // An implementation may elide the menu when only one assignment is legal.
    assert_eq!(
        game.player(alice).unwrap().life,
        if cannot_gain { 10 } else { 20 }
    );
    assert_eq!(
        game.player(bob).unwrap().life,
        if cannot_gain { 20 } else { 10 }
    );
}

#[test]
fn cr_119_7_redistribution_cannot_assign_a_higher_total_to_a_player_who_cannot_gain() {
    restricted_redistribution(true);
}

#[test]
fn cr_119_8_redistribution_cannot_assign_a_lower_total_to_a_player_who_cannot_lose() {
    restricted_redistribution(false);
}

#[test]
fn cr_102_2_only_the_other_player_is_an_opponent() {
    let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for a in 0..2 {
        for b in 0..2 {
            assert_eq!(
                game.are_opponents(PlayerId::from_index(a), PlayerId::from_index(b)),
                a != b
            );
        }
    }
}

#[test]
fn cr_102_3_team_membership_defines_teammates_and_opponents() {
    let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    game.set_teams(vec![vec![p[0], p[2]], vec![p[1], p[3]]])
        .unwrap();
    for a in 0..4 {
        for b in 0..4 {
            assert_eq!(game.are_teammates(p[a], p[b]), a != b && a % 2 == b % 2);
            assert_eq!(game.are_opponents(p[a], p[b]), a % 2 != b % 2);
        }
    }
}

#[test]
fn cr_119_4b_zero_life_payment_is_legal_even_at_negative_life_and_under_prohibition() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    for life in [-10, 0, 10] {
        game.player_mut(alice).unwrap().life = life;
        game.effect_store.cant_effects.cant_lose_life.insert(alice);
        game.effect_store
            .cant_effects
            .life_total_cant_change
            .insert(alice);
        assert!(game.can_pay_life(alice, 0));
        assert!(!game.can_pay_life(alice, 1));
    }
}

#[test]
fn cr_119_5_setting_life_gains_or_loses_the_difference_including_negative_totals() {
    use ironsmith::effects::SetLifeTotalEffect;
    use ironsmith::events::{LifeGainEvent, LifeLossEvent};
    let alice = PlayerId::from_index(0);
    for (old, new) in [(10, 15), (10, 5), (-5, -10), (-10, -5), (10, -5)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.player_mut(alice).unwrap().life = old;
        let source = game.new_object_id();
        let mut ctx = EffectContext::new_default(source, alice);
        let outcome = SetLifeTotalEffect::you(new)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, new);
        let gained: u32 = outcome
            .events
            .iter()
            .filter_map(|e| e.downcast::<LifeGainEvent>())
            .map(|e| e.amount)
            .sum();
        let lost: u32 = outcome
            .events
            .iter()
            .filter_map(|e| e.downcast::<LifeLossEvent>())
            .map(|e| e.amount)
            .sum();
        assert_eq!(gained, if new > old { new.abs_diff(old) } else { 0 });
        assert_eq!(lost, if new < old { new.abs_diff(old) } else { 0 });
    }
}

#[test]
fn cr_119_4_positive_life_payments_require_sufficient_life_and_subtract_the_payment() {
    let alice = PlayerId::from_index(0);
    for (life, payment, allowed) in [
        (-1, 1, false),
        (0, 1, false),
        (3, 4, false),
        (3, 3, true),
        (3, 2, true),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.player_mut(alice).unwrap().life = life;
        assert_eq!(game.can_pay_life(alice, payment), allowed);
        assert_eq!(game.pay_life(alice, payment).unwrap(), allowed);
        assert_eq!(
            game.player(alice).unwrap().life,
            if allowed { life - payment as i32 } else { life }
        );
    }
}

#[test]
fn cr_119_4a_two_headed_giant_life_payments_use_the_shared_aggregate() {
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    for (payments, allowed) in [
        (vec![(p[0], 20), (p[1], 11)], false),
        (vec![(p[0], 20), (p[1], 10)], true),
    ] {
        let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
        game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
            .unwrap();
        assert_eq!(game.can_pay_life_simultaneously(&payments), allowed);
        assert_eq!(game.pay_life_simultaneously(&payments).unwrap(), allowed);
        for player in &p[..2] {
            assert_eq!(
                game.player(*player).unwrap().life,
                if allowed { 0 } else { 30 }
            );
        }
        assert_eq!(game.player(p[2]).unwrap().life, 30);
    }
}

#[test]
fn cr_119_6_zero_and_negative_life_produce_a_state_based_loss() {
    use ironsmith::rules::state_based::{LoseReason, StateBasedAction};
    let alice = PlayerId::from_index(0);
    for life in [-10, 0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.player_mut(alice).unwrap().life = life;
        let actions = ironsmith::check_state_based_actions(&game);
        assert_eq!(actions.iter().any(|a| matches!(a,
            StateBasedAction::PlayerLoses { player, reason: LoseReason::ZeroLife } if *player == alice
        )), life <= 0);
    }
}

#[test]
fn cr_119_9_zero_life_gain_emits_no_gain_event() {
    use ironsmith::effects::GainLifeEffect;
    use ironsmith::events::LifeGainEvent;
    use ironsmith::{ChooseSpec, Value};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, alice);
    let outcome = GainLifeEffect {
        amount: Value::Fixed(0),
        player: ChooseSpec::Player(PlayerFilter::You),
    }
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        !outcome
            .events
            .iter()
            .any(|e| e.downcast::<LifeGainEvent>().is_some())
    );
    assert!(
        !game
            .take_pending_trigger_events()
            .iter()
            .any(|e| e.downcast::<LifeGainEvent>().is_some())
    );
}

#[test]
fn cr_119_10_zero_gain_does_not_apply_a_gain_replacement() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::GainLifeEffect;
    use ironsmith::events::life::matchers::WouldGainLifeMatcher;
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use ironsmith::{CardId, CardType, ChooseSpec, Value, Zone};
    let alice = PlayerId::from_index(0);
    for amount in [0, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Life replacement source")
            .card_types(vec![CardType::Enchantment])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGainLifeMatcher::you(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
        );
        let mut ctx = EffectContext::new_default(source, alice);
        GainLifeEffect {
            amount: Value::Fixed(amount),
            player: ChooseSpec::Player(PlayerFilter::You),
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            if amount == 0 { 20 } else { 23 },
            "zero gain must bypass replacements; the positive control must apply the replacement"
        );
    }
}

#[test]
fn cr_121_5_moving_a_library_card_to_hand_is_not_a_draw() {
    use ironsmith::effects::{DrawCardsEffect, MoveToZoneEffect};
    use ironsmith::events::{CardsDrawnEvent, cards::matchers::WouldDrawCardMatcher};
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{ChooseSpec, Effect, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let card = counter_creature(&mut game, Zone::Library);
    let identity = game.object(card).unwrap().card;
    game.effect_store
        .replacement_effects
        .add_resolution_effect(ReplacementEffect::with_matcher(
            source,
            alice,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![Effect::gain_life(5)]),
        ));
    let mut ctx = EffectContext::new_default(source, alice);
    let outcome = MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Hand, false)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(
        game.object(game.player(alice).unwrap().hand[0])
            .unwrap()
            .card,
        identity
    );
    assert_eq!(
        game.player(alice).unwrap().life,
        20,
        "CR 121.5: draw replacement must not apply"
    );
    assert_eq!(outcome.events_of_type::<CardsDrawnEvent>().count(), 0);
    assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(alice), 0);
    assert!(
        !game
            .player(alice)
            .unwrap()
            .attempted_draw_from_empty_library
    );
    // The same replacement really applies to a subsequent draw, even from this empty library.
    DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(game.player(alice).unwrap().life, 25);
}

#[test]
fn cr_121_2c_each_player_finishes_drawing_in_active_player_turn_order() {
    use ironsmith::effects::ForPlayersEffect;
    use ironsmith::events::CardsDrawnEvent;
    use ironsmith::{Effect, Zone};
    for active_index in 0..3 {
        let players = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
        ];
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        game.turn.active_player = players[active_index];
        game.turn_store.turn_order = players.to_vec();
        let source = counter_creature(&mut game, Zone::Battlefield);
        for player in players {
            for _ in 0..2 {
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Draw card",
                )
                .card_types(vec![ironsmith::CardType::Artifact])
                .build();
                game.create_object_from_definition(&definition, player, Zone::Library);
            }
        }
        let outcome = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::target_draws(2, PlayerFilter::IteratedPlayer)],
        )
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, players[0]),
        )
        .unwrap();
        let order: Vec<_> = outcome
            .events_of_type::<CardsDrawnEvent>()
            .flat_map(|e| std::iter::repeat_n(e.player, e.amount() as usize))
            .collect();
        let expected: Vec<_> = (0..3)
            .flat_map(|offset| [players[(active_index + offset) % 3]; 2])
            .collect();
        assert_eq!(
            order, expected,
            "CR 121.2c: finish each player's draws in APNAP order"
        );
        for player in players {
            assert_eq!(game.player(player).unwrap().hand.len(), 2);
        }
    }
}

#[test]
fn cr_121_2d_shared_team_draws_follow_each_teams_chosen_member_order() {
    use ironsmith::effects::ForPlayersEffect;
    use ironsmith::events::CardsDrawnEvent;
    use ironsmith::{Effect, Zone};
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
    game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
        .unwrap();
    game.turn.active_player = p[0];
    game.set_shared_team_member_order(0, vec![p[1], p[0]])
        .unwrap();
    game.set_shared_team_member_order(1, vec![p[3], p[2]])
        .unwrap();
    assert_eq!(game.team_apnap_player_order(), vec![p[1], p[0], p[3], p[2]]);
    let source = counter_creature(&mut game, Zone::Battlefield);
    for player in &p {
        for _ in 0..2 {
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Team draw card",
            )
            .card_types(vec![ironsmith::CardType::Artifact])
            .build();
            game.create_object_from_definition(&definition, *player, Zone::Library);
        }
    }
    let outcome = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::target_draws(2, PlayerFilter::IteratedPlayer)],
    )
    .execute(&mut game, &mut EffectContext::new_default(source, p[0]))
    .unwrap();
    let order: Vec<_> = outcome
        .events_of_type::<CardsDrawnEvent>()
        .flat_map(|e| std::iter::repeat_n(e.player, e.amount() as usize))
        .collect();
    assert_eq!(
        order,
        vec![p[1], p[1], p[0], p[0], p[3], p[3], p[2], p[2]],
        "CR 121.2d: each team chooses its members' draw order, and each member completes all draws"
    );
}

#[test]
fn cr_121_1_draws_move_the_top_cards_in_order_and_record_each_draw() {
    use ironsmith::Zone;
    use ironsmith::effects::DrawCardsEffect;
    use ironsmith::events::CardsDrawnEvent;
    for count in [0, 1, 3] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        for _ in 0..4 {
            counter_creature(&mut game, Zone::Library);
        }
        let expected: Vec<_> = game
            .player(alice)
            .unwrap()
            .library
            .iter()
            .rev()
            .take(count)
            .map(|id| game.object(*id).unwrap().card)
            .collect();
        let outcome = DrawCardsEffect::you(count as i32)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let actual: Vec<_> = outcome
            .events_of_type::<CardsDrawnEvent>()
            .flat_map(|event| event.cards.iter())
            .map(|id| {
                let card = game.object(*id).unwrap();
                assert_eq!(card.zone, Zone::Hand);
                card.card
            })
            .collect();
        assert_eq!(
            actual, expected,
            "CR 121.1: draw from the top, preserving draw order"
        );
        assert_eq!(game.player(alice).unwrap().hand.len(), count);
        assert_eq!(game.player(alice).unwrap().library.len(), 4 - count);
        assert_eq!(
            game.turn_store.turn_history.cards_drawn_by_player(alice),
            count as u32
        );
    }
}

#[test]
fn cr_122_1f_poison_loss_occurs_at_the_correct_individual_or_team_sba_threshold() {
    use ironsmith::Zone;
    use ironsmith::effects::PoisonCountersEffect;
    use ironsmith::rules::state_based::{LoseReason, StateBasedAction};
    for shared in [false, true] {
        for count in [0, 1, 9, 10, 14, 15, 16] {
            let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
            let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
            if shared {
                game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
                    .unwrap();
            }
            let source = counter_creature(&mut game, Zone::Battlefield);
            PoisonCountersEffect::new(count, PlayerFilter::Specific(p[0]))
                .execute(&mut game, &mut EffectContext::new_default(source, p[2]))
                .unwrap();
            assert_eq!(game.player(p[0]).unwrap().poison_counters, count as u32);
            assert!(
                game.player(p[0]).unwrap().is_in_game(),
                "Loss waits for state-based actions"
            );
            let lethal = count >= if shared { 15 } else { 10 };
            assert_eq!(
                ironsmith::check_state_based_actions(&game)
                    .iter()
                    .any(|a| matches!(
                        a,
                        StateBasedAction::PlayerLoses {
                            reason: LoseReason::Poison,
                            ..
                        }
                    )),
                lethal
            );
            ironsmith::apply_state_based_actions(&mut game).unwrap();
            assert_eq!(game.player(p[0]).unwrap().is_in_game(), !lethal);
            assert_eq!(game.player(p[1]).unwrap().is_in_game(), !(shared && lethal));
            assert!(game.player(p[2]).unwrap().is_in_game());
            assert!(game.player(p[3]).unwrap().is_in_game());
        }
    }
}

#[test]
fn cr_810_10_shared_poison_adds_per_player_but_counts_each_team_once() {
    use ironsmith::effects::{ForPlayersEffect, PoisonCountersEffect};
    use ironsmith::{CounterType, Effect, Value, Zone};
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    let mut game = GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
    game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
        .unwrap();
    let source = counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, p[0]);
    ForPlayersEffect::new(
        PlayerFilter::Opponent,
        vec![Effect::new(PoisonCountersEffect::new(
            2,
            PlayerFilter::IteratedPlayer,
        ))],
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    for player in &p[2..] {
        assert_eq!(
            game.player(*player).unwrap().poison_counters,
            4,
            "CR 810.10: two individual placements accumulate in the shared pool"
        );
    }
    let value = Value::PlayerCounters(PlayerFilter::Opponent, CounterType::Poison);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        4,
        "CR 810.10a: opposing team poison must not be counted once per head"
    );
    let removed = game
        .remove_player_counters_with_source(p[3], CounterType::Poison, 1, Some(source), Some(p[0]))
        .unwrap();
    assert_eq!(removed.0, 1);
    for player in &p[2..] {
        assert_eq!(game.player(*player).unwrap().poison_counters, 3);
    }
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        3
    );
}

fn audit_sticker_ownership(action: ironsmith::events::KeywordActionKind) {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::{Ability, ChooseSpec, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for owner in [alice, bob] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Sticker recipient",
        )
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
        let recipient = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        game.add_accessible_art_sticker(alice);
        game.add_accessible_name_sticker(alice, "Bright");
        game.add_accessible_pt_sticker(alice, 0, 6, 7);
        game.add_accessible_ability_sticker(
            alice,
            0,
            vec![Ability::static_ability(StaticAbility::flying())],
            "Flying",
        );
        PutStickerEffect::new(ChooseSpec::SpecificObject(recipient), action)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        assert_eq!(
            game.sticker_count_on_object(recipient, action, None),
            u32::from(owner == alice),
            "CR 123.3b: Alice may place her sticker only on an object she owns; action={action:?}"
        );
    }
}
#[test]
fn cr_123_3b_art_stickers_cannot_be_placed_on_another_players_object() {
    audit_sticker_ownership(ironsmith::events::KeywordActionKind::ArtSticker);
}
#[test]
fn cr_123_3b_ability_stickers_cannot_be_placed_on_another_players_object() {
    audit_sticker_ownership(ironsmith::events::KeywordActionKind::AbilitySticker);
}
#[test]
fn cr_123_3b_pt_stickers_cannot_be_placed_on_another_players_object() {
    audit_sticker_ownership(ironsmith::events::KeywordActionKind::PowerToughnessSticker);
}
#[test]
fn cr_123_3b_name_stickers_respect_object_ownership() {
    audit_sticker_ownership(ironsmith::events::KeywordActionKind::NameSticker);
}

#[test]
fn cr_123_3_no_ability_sticker_can_be_placed_without_an_available_sticker() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    assert!(game.available_ability_stickers(alice).is_empty());
    PutStickerEffect::new(
        ChooseSpec::SpecificObject(source),
        KeywordActionKind::AbilitySticker,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert_eq!(
        game.sticker_count_on_object(source, KeywordActionKind::Sticker, None),
        0,
        "CR 123.3: an instruction cannot manufacture a sticker absent from accessible sheets"
    );
}

#[test]
fn cr_123_5_name_sticker_follows_public_moves_but_returns_to_sheet_after_hidden_move() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for destination in [
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
        Zone::Stack,
        Zone::Hand,
        Zone::Library,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let sticker = game.add_accessible_name_sticker(alice, "Bright");
        PutStickerEffect::new(
            ChooseSpec::SpecificObject(source),
            KeywordActionKind::NameSticker,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert_eq!(
            game.current_name(source).as_deref(),
            Some("Bright Counter recipient")
        );
        assert!(game.available_name_stickers(alice).is_empty());
        let moved = game.move_object_by_effect(source, destination).unwrap();
        let retained = !matches!(destination, Zone::Hand | Zone::Library);
        assert_eq!(
            game.sticker_count_on_object(moved, KeywordActionKind::Sticker, None),
            u32::from(retained),
            "CR 123.5: sticker retention for {destination:?}"
        );
        assert_eq!(
            game.current_name(moved).as_deref(),
            Some(if retained {
                "Bright Counter recipient"
            } else {
                "Counter recipient"
            })
        );
        assert_eq!(
            game.available_name_stickers(alice)
                .iter()
                .any(|entry| entry.0 == sticker),
            !retained
        );
    }
}

#[test]
fn cr_123_3a_identical_name_stickers_are_distinct_and_cannot_be_reused_while_placed() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let first = game.add_accessible_name_sticker(alice, "Bright");
    let second = game.add_accessible_name_sticker(alice, "Bright");
    assert_ne!(first, second);
    let effect = PutStickerEffect::new(
        ChooseSpec::SpecificObject(source),
        KeywordActionKind::NameSticker,
    );
    for placed in 1..=3 {
        effect
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        assert_eq!(
            game.sticker_count_on_object(source, KeywordActionKind::Sticker, None),
            placed.min(2)
        );
        assert_eq!(
            game.available_name_stickers(alice).len(),
            2 - placed.min(2) as usize
        );
    }
    assert_eq!(
        game.current_name(source).as_deref(),
        Some("Bright Bright Counter recipient")
    );
}

#[test]
fn cr_123_3c_paid_stickers_require_and_spend_their_ticket_cost() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, ChooseSpec, CounterType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    for action in [
        KeywordActionKind::AbilitySticker,
        KeywordActionKind::PowerToughnessSticker,
    ] {
        for balance in [0, 1, 2, 3] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = counter_creature(&mut game, Zone::Battlefield);
            let ticket = CounterType::Named("ticket".into());
            game.add_player_counters_with_source(alice, ticket, balance, Some(source), Some(alice))
                .unwrap();
            game.add_accessible_ability_sticker(
                alice,
                2,
                vec![Ability::static_ability(StaticAbility::flying())],
                "Flying",
            );
            game.add_accessible_pt_sticker(alice, 2, 6, 7);
            PutStickerEffect::new(ChooseSpec::SpecificObject(source), action)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            assert_eq!(
                game.sticker_count_on_object(source, action, None),
                u32::from(balance >= 2)
            );
            assert_eq!(
                game.player(alice).unwrap().counter_count(ticket),
                if balance >= 2 { balance - 2 } else { balance }
            );
        }
    }
}

#[test]
fn cr_123_8_pt_stickers_set_creature_card_stats_in_public_zones_in_timestamp_order() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for zone in [
        Zone::Battlefield,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, zone);
        for (power, toughness) in [(6, 7), (1, 2)] {
            game.add_accessible_pt_sticker(alice, 0, power, toughness);
            PutStickerEffect::new(
                ChooseSpec::SpecificObject(source),
                KeywordActionKind::PowerToughnessSticker,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            assert_eq!(
                game.current_power(source),
                Some(power),
                "CR 123.8: sticker power in {zone:?}"
            );
            assert_eq!(
                game.current_toughness(source),
                Some(toughness),
                "CR 123.8: sticker toughness in {zone:?}"
            );
        }
    }
}

#[test]
fn cr_123_6d_name_sticker_letter_counts_ignore_case() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    game.add_accessible_name_sticker(alice, "AaA-bB");
    PutStickerEffect::new(
        ChooseSpec::SpecificObject(source),
        KeywordActionKind::NameSticker,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    for (letter, expected) in [('a', 3), ('A', 3), ('b', 2), ('B', 2), ('z', 0)] {
        assert_eq!(
            game.name_sticker_character_count_on_object(source, letter),
            expected
        );
    }
}

#[test]
fn cr_123_6e_unique_vowels_include_y_and_ignore_repetition_and_case() {
    for (name, expected) in [
        ("", 0),
        ("bcdf", 0),
        ("YyY", 1),
        ("AaEEiiOOuUyY", 6),
        ("Queue", 2),
        ("Rhythm", 1),
    ] {
        assert_eq!(
            ironsmith::game_state::name_sticker_unique_vowels(name),
            expected,
            "CR 123.6e: {name}"
        );
    }
}

struct ChooseNamePosition {
    owner: PlayerId,
    controller: PlayerId,
    words: usize,
    position: usize,
    seen: usize,
}
impl DecisionMaker for ChooseNamePosition {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        self.seen += 1;
        if ctx.description == "Choose a name sticker" {
            assert_eq!(ctx.player, self.owner);
            vec![0]
        } else {
            assert_eq!(ctx.description, "Choose the name sticker's position");
            assert_eq!(
                ctx.player, self.controller,
                "CR 123.6b: the object's controller chooses the position"
            );
            assert_eq!(
                ctx.options.len(),
                self.words + 1,
                "CR 123.6a: punctuation and hyphens do not split words; blank lines are not words"
            );
            vec![self.position]
        }
    }
}

#[test]
fn cr_123_6a_and_b_name_position_choices_use_controller_and_rule_word_boundaries() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, words, position, expected) in [
        ("Bear Cub", 2, 1, Some("Bear Bright Cub")),
        ("Hyphen-name, Last", 2, 2, Some("Hyphen-name, Last Bright")),
        ("", 0, 0, Some("Bright")),
        ("Wolf in ________ Clothing", 3, 1, None),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
                .card_types(vec![ironsmith::CardType::Artifact])
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::ChangeController(bob),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert_eq!(game.current_controller(source), Some(bob));
        game.add_accessible_name_sticker(alice, "Bright");
        let mut choices = ChooseNamePosition {
            owner: alice,
            controller: bob,
            words,
            position,
            seen: 0,
        };
        PutStickerEffect::new(
            ChooseSpec::SpecificObject(source),
            KeywordActionKind::NameSticker,
        )
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, alice).with_decision_maker(&mut choices),
        )
        .unwrap();
        assert_eq!(choices.seen, 2);
        if let Some(expected) = expected {
            assert_eq!(game.current_name(source).as_deref(), Some(expected));
        }
    }
}

#[test]
fn cr_123_6c_later_name_setting_overrides_older_name_sticker() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    game.add_accessible_name_sticker(alice, "Bright");
    PutStickerEffect::new(
        ChooseSpec::SpecificObject(source),
        KeywordActionKind::NameSticker,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert_eq!(
        game.current_name(source).as_deref(),
        Some("Bright Counter recipient")
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::SetName("Legitimate Businessperson".into()),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert_eq!(
        game.current_name(source).as_deref(),
        Some("Legitimate Businessperson"),
        "CR 123.6c: later text-changing name effect overrides the sticker"
    );
    assert_eq!(
        game.sticker_count_on_object(source, KeywordActionKind::NameSticker, None),
        1
    );
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(
        game.current_name(source).as_deref(),
        Some("Bright Counter recipient")
    );
}

#[test]
fn cr_123_7_ability_sticker_grants_follow_public_zone_changes_and_end_in_hidden_zones() {
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, ChooseSpec, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    for destination in [
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
        Zone::Hand,
        Zone::Library,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let sticker = game.add_accessible_ability_sticker(
            alice,
            0,
            vec![
                Ability::static_ability(StaticAbility::flying()),
                Ability::static_ability(StaticAbility::lifelink()),
            ],
            "Flying, lifelink",
        );
        PutStickerEffect::new(
            ChooseSpec::SpecificObject(source),
            KeywordActionKind::AbilitySticker,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert!(game.object_has_ability(source, &StaticAbility::flying()));
        assert!(game.object_has_ability(source, &StaticAbility::lifelink()));
        let moved = game.move_object_by_effect(source, destination).unwrap();
        let retained = !matches!(destination, Zone::Hand | Zone::Library);
        assert_eq!(
            game.object_has_ability(moved, &StaticAbility::flying()),
            retained,
            "CR 123.7 / 123.5: flying in {destination:?}"
        );
        assert_eq!(
            game.object_has_ability(moved, &StaticAbility::lifelink()),
            retained,
            "CR 123.7 / 123.5: lifelink in {destination:?}"
        );
        assert_eq!(
            game.available_ability_stickers(alice)
                .iter()
                .any(|entry| entry.id == sticker),
            !retained
        );
    }
}

#[test]
fn cr_123_7a_sticker_ability_reference_remains_when_ability_is_removed() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, ChooseSpec, StaticAbility, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let ability = Ability::static_ability(StaticAbility::flying());
    game.add_accessible_ability_sticker(alice, 0, vec![ability.clone()], "Flying");
    PutStickerEffect::new(
        ChooseSpec::SpecificObject(source),
        KeywordActionKind::AbilitySticker,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::RemoveAllAbilities,
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert!(!game.object_has_ability(source, &StaticAbility::flying()));
    assert_eq!(
        game.ability_sticker_abilities_on_object(source),
        vec![ability],
        "CR 123.7a: sticker reference retains its printed ability"
    );
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(game.object_has_ability(source, &StaticAbility::flying()));
}

#[test]
fn cr_123_1_copy_excludes_all_four_kinds_of_stickers() {
    use ironsmith::effects::{CreateTokenCopyEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, ChooseSpec, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    game.add_accessible_art_sticker(alice);
    game.add_accessible_name_sticker(alice, "Bright");
    game.add_accessible_pt_sticker(alice, 0, 6, 7);
    game.add_accessible_ability_sticker(
        alice,
        0,
        vec![Ability::static_ability(StaticAbility::flying())],
        "Flying",
    );
    for action in [
        KeywordActionKind::ArtSticker,
        KeywordActionKind::NameSticker,
        KeywordActionKind::PowerToughnessSticker,
        KeywordActionKind::AbilitySticker,
    ] {
        PutStickerEffect::new(ChooseSpec::SpecificObject(source), action)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
    }
    assert_eq!(
        game.current_name(source).as_deref(),
        Some("Bright Counter recipient")
    );
    assert_eq!(game.current_power(source), Some(6));
    assert!(game.object_has_ability(source, &StaticAbility::flying()));
    assert_eq!(
        game.sticker_count_on_object(source, KeywordActionKind::Sticker, None),
        4
    );
    CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    let copies: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| *id != source)
        .collect();
    assert_eq!(copies.len(), 1);
    let copy = copies[0];
    assert_eq!(
        game.current_name(copy).as_deref(),
        Some("Counter recipient"),
        "CR 123.1: name sticker is not copiable"
    );
    assert_eq!(game.current_power(copy), Some(3));
    assert_eq!(game.current_toughness(copy), Some(4));
    assert!(!game.object_has_ability(copy, &StaticAbility::flying()));
    assert_eq!(
        game.sticker_count_on_object(copy, KeywordActionKind::Sticker, None),
        0
    );
}

#[test]
fn cr_201_2a_and_c_name_filters_distinguish_nameless_objects_from_different_names() {
    use ironsmith::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    for candidate_name in [Some("Alpha"), Some("Beta"), None] {
        for reference_names in [
            vec![Some("Alpha")],
            vec![Some("Beta")],
            vec![None],
            vec![Some("Alpha"), Some("Beta")],
            vec![None, Some("Alpha")],
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut create = |name: Option<&str>| {
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    name.unwrap_or("Hidden identity"),
                )
                .card_types(vec![ironsmith::CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
                let id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                if name.is_none() {
                    assert!(game.set_face_down(id));
                }
                id
            };
            let candidate = create(candidate_name);
            let references: Vec<_> = reference_names.iter().map(|name| create(*name)).collect();
            game.refresh_continuous_state().unwrap();
            let mut ctx = EffectContext::new_default(candidate, alice);
            ctx.tagged_objects.insert(
                "reference".into(),
                references
                    .iter()
                    .map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))
                    .collect(),
            );
            let shared = candidate_name.is_some() && reference_names.contains(&candidate_name);
            for (relation, expected) in [
                (TaggedOpbjectRelation::SameNameAsTagged, shared),
                (
                    TaggedOpbjectRelation::DifferentNameFromTagged,
                    candidate_name.is_some() && !shared,
                ),
            ] {
                let mut filter = ObjectFilter::default();
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: "reference".into(),
                    relation,
                });
                assert_eq!(
                    ironsmith::effects::helpers::resolve_objects_from_spec(
                        &game,
                        &ironsmith::ChooseSpec::All(filter),
                        &ctx
                    )
                    .unwrap()
                    .contains(&candidate),
                    expected,
                    "CR 201.2a/c: candidate={candidate_name:?}, references={reference_names:?}"
                );
            }
        }
    }
}

#[test]
fn cr_201_2b_distinct_name_count_excludes_nameless_permanents() {
    use ironsmith::{ObjectFilter, Value, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let first = counter_creature(&mut game, Zone::Battlefield);
    counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(first, alice);
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Battlefield);
    let value = Value::DistinctNames(filter);
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(),
        1
    );
    let nameless = counter_creature(&mut game, Zone::Battlefield);
    assert!(game.set_face_down(nameless));
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        ironsmith::effects::helpers::resolve_value(&game, &value, &mut ctx).unwrap(),
        1,
        "CR 201.2b: a nameless object cannot contribute a different name"
    );
}

#[test]
fn cr_201_2b_group_selection_requires_every_object_to_have_a_distinct_name() {
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::{ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut ids = Vec::new();
    for name in ["Alpha", "Beta", "Alpha", "Hidden name"] {
        let definition =
            ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
                .card_types(vec![ironsmith::CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
        ids.push(game.create_object_from_definition(&definition, alice, Zone::Battlefield));
    }
    assert!(game.set_face_down(ids[3]));
    game.refresh_continuous_state().unwrap();
    let mut filter = ObjectFilter::default();
    filter.distinct_names = true;
    let choice = SelectObjectsContext::new(
        alice,
        Some(ids[0]),
        "Choose different names",
        vec![],
        2,
        Some(3),
    )
    .with_relation_filter(filter);
    for (selection, allowed) in [
        (vec![ids[0], ids[1]], true),
        (vec![ids[0], ids[2]], false),
        (vec![ids[0], ids[3]], false),
        (vec![ids[0], ids[1], ids[3]], false),
    ] {
        assert_eq!(
            choice.selection_satisfies_relation_filter(&game, &selection),
            allowed
        );
    }
}

struct SubmitCardName(&'static str);
impl DecisionMaker for SubmitCardName {
    fn decide_text(
        &mut self,
        _game: &GameState,
        _ctx: &ironsmith::decisions::context::TextInputContext,
    ) -> String {
        self.0.into()
    }
}

#[test]
fn cr_201_4a_choose_artifact_name_cannot_accept_island() {
    use ironsmith::effects::ChooseCardNameEffect;
    use ironsmith::{ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    for restricted in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let mut choice = SubmitCardName("Island");
        let effect = ChooseCardNameEffect::new(
            PlayerFilter::You,
            restricted.then(ObjectFilter::artifact),
            "chosen_name",
        );
        let _ = effect.execute(
            &mut game,
            &mut EffectContext::new_default(source, alice).with_decision_maker(&mut choice),
        );
        assert_eq!(
            game.chosen_named_option(source),
            if restricted { None } else { Some("Island") },
            "CR 201.4a: Island is a card name but not an artifact card name"
        );
    }
}

#[test]
fn cr_201_5_runtime_self_reference_survives_a_name_change_without_selecting_a_namesake() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, UntapEffect};
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let namesake = counter_creature(&mut game, Zone::Battlefield);
    game.tap(source);
    game.tap(namesake);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::SetName("Changed name".into()),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert_ne!(game.current_name(source), game.current_name(namesake));
    UntapEffect::target(ChooseSpec::Source)
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    assert!(!game.is_tapped(source));
    assert!(
        game.is_tapped(namesake),
        "CR 201.5: an identically printed name is not a self-reference target"
    );
}

#[test]
fn cr_202_2_mana_symbol_colors_determine_every_ordinary_color_combination() {
    use ironsmith::{ColorSet, ManaCost, ManaSymbol, Zone};
    let colors = [
        (ManaSymbol::White, ColorSet::WHITE),
        (ManaSymbol::Blue, ColorSet::BLUE),
        (ManaSymbol::Black, ColorSet::BLACK),
        (ManaSymbol::Red, ColorSet::RED),
        (ManaSymbol::Green, ColorSet::GREEN),
    ];
    for mask in 0..32 {
        for zone in [
            Zone::Battlefield,
            Zone::Hand,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Library,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut symbols = vec![
                ManaSymbol::Generic(2),
                ManaSymbol::Colorless,
                ManaSymbol::Snow,
            ];
            let mut expected = ColorSet::default();
            for (bit, (symbol, color)) in colors.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    symbols.push(*symbol);
                    expected = expected.union(*color);
                }
            }
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Color subject",
            )
            .card_types(vec![ironsmith::CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(symbols))
            .build();
            let object =
                game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
            assert_eq!(
                game.current_colors(object),
                Some(expected),
                "CR 202.2a-c: colors mask={mask} zone={zone:?}"
            );
        }
    }
}

#[test]
fn cr_202_2e_and_f_color_indicator_and_color_changing_effects_apply() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{ColorSet, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Indicator subject",
    )
    .card_types(vec![ironsmith::CardType::Creature])
    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
    .color_indicator(ColorSet::RED.union(ColorSet::GREEN))
    .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert_eq!(
        game.current_colors(source),
        Some(ColorSet::RED.union(ColorSet::GREEN))
    );
    for color in [ColorSet::default(), ColorSet::BLUE] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::SetColors(color),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert_eq!(game.current_colors(source), Some(color));
    }
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(
        game.current_colors(source),
        Some(ColorSet::RED.union(ColorSet::GREEN))
    );
}

#[test]
fn cr_202_2d_and_202_3f_g_hybrid_and_phyrexian_symbols_keep_colors_and_mana_value() {
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{ColorSet, ManaCost, ManaSymbol, Zone};
    let colors = [
        (ManaSymbol::White, ColorSet::WHITE),
        (ManaSymbol::Blue, ColorSet::BLUE),
        (ManaSymbol::Black, ColorSet::BLACK),
        (ManaSymbol::Red, ColorSet::RED),
        (ManaSymbol::Green, ColorSet::GREEN),
    ];
    let mut cases = Vec::new();
    for (i, (symbol, color)) in colors.iter().enumerate() {
        cases.push((vec![*symbol, ManaSymbol::Generic(2)], *color, 2));
        cases.push((vec![*symbol, ManaSymbol::Life(2)], *color, 1));
        for (other, other_color) in colors.iter().skip(i + 1) {
            cases.push((vec![*symbol, *other], color.union(*other_color), 1));
            cases.push((
                vec![*symbol, *other, ManaSymbol::Life(2)],
                color.union(*other_color),
                1,
            ));
        }
    }
    for (pip, expected_color, pip_value) in cases {
        for zone in [
            Zone::Battlefield,
            Zone::Hand,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Library,
            Zone::Command,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Hybrid subject",
            )
            .card_types(vec![ironsmith::CardType::Artifact])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                pip.clone(),
                pip.clone(),
            ]))
            .build();
            let object =
                game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
            assert_eq!(
                game.current_colors(object),
                Some(expected_color),
                "CR 202.2d: {pip:?} in {zone:?}"
            );
            assert_eq!(
                ObjectSnapshot::from_object(game.object(object).unwrap(), &game).mana_value(),
                1 + 2 * pip_value,
                "CR 202.3f/g: {pip:?} in {zone:?}"
            );
        }
    }
}

#[test]
fn cr_202_3e_each_x_contributes_its_chosen_value_only_on_the_stack() {
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{ManaCost, ManaSymbol, Zone};
    for x in [0, 1, 4] {
        for zone in [
            Zone::Battlefield,
            Zone::Hand,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Library,
            Zone::Command,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Double X subject",
            )
            .card_types(vec![ironsmith::CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::X,
                ManaSymbol::X,
                ManaSymbol::Red,
            ]))
            .build();
            let object =
                game.create_object_from_definition(&definition, PlayerId::from_index(0), zone);
            game.object_mut(object).unwrap().x_value = Some(x);
            assert_eq!(
                ObjectSnapshot::from_object(game.object(object).unwrap(), &game).mana_value(),
                if zone == Zone::Stack { 1 + 2 * x } else { 1 },
                "CR 202.3e: X={x}, zone={zone:?}"
            );
        }
    }
}

#[test]
fn cr_202_3b_transform_retains_front_mana_value_but_copy_of_back_has_zero() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::{ApplyContinuousEffect, RuntimeModification, TransformEffect};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{
        CardId, CardType, ChooseSpec, ManaCost, ManaSymbol, PowerToughness, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut front = CardDefinitionBuilder::new(CardId::new(), "Transform front")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Red,
            ManaSymbol::Green,
        ]))
        .build();
    let mut back = CardDefinitionBuilder::new(CardId::new(), "Transform back")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    let original = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    assert_eq!(
        ObjectSnapshot::from_object(game.object(original).unwrap(), &game).mana_value(),
        4
    );
    TransformEffect::source()
        .execute(&mut game, &mut EffectContext::new_default(original, alice))
        .unwrap();
    assert_eq!(
        game.current_name(original).as_deref(),
        Some("Transform back")
    );
    assert_eq!(
        ObjectSnapshot::from_object(game.object(original).unwrap(), &game).mana_value(),
        4,
        "CR 202.3b: transformed original retains its front-face mana value"
    );
    let copy = counter_creature(&mut game, Zone::Battlefield);
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(copy),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(original),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(copy, alice))
    .unwrap();
    assert_eq!(game.current_name(copy).as_deref(), Some("Transform back"));
    assert_eq!(
        ObjectSnapshot::from_object(game.object(copy).unwrap(), &game).mana_value(),
        0,
        "CR 202.3b: a single-faced permanent copying the back face has mana value zero"
    );
}

#[test]
fn cr_202_3d_split_card_mana_value_combines_halves_only_outside_the_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::{CardId, CardType, ManaCost, ManaSymbol, Zone};
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
        Zone::Stack,
    ] {
        for use_back in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut front = CardDefinitionBuilder::new(CardId::new(), "Split first")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(1),
                    ManaSymbol::Red,
                ]))
                .build();
            let mut back = CardDefinitionBuilder::new(CardId::new(), "Split second")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(2),
                    ManaSymbol::Blue,
                ]))
                .build();
            front.card.other_face = Some(back.card.id);
            front.card.other_face_name = Some(back.card.name.to_string());
            front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
            back.card.other_face = Some(front.card.id);
            back.card.other_face_name = Some(front.card.name.to_string());
            back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let object = game.create_object_from_definition(
                if use_back { &back } else { &front },
                PlayerId::from_index(0),
                zone,
            );
            let expected = if zone == Zone::Stack {
                if use_back { 3 } else { 2 }
            } else {
                5
            };
            assert_eq!(
                ObjectSnapshot::from_object(game.object(object).unwrap(), &game).mana_value(),
                expected,
                "CR 202.3d: zone={zone:?}, second half={use_back}"
            );
        }
    }
}

#[test]
fn cr_205_3d_cannot_add_a_creature_subtype_to_a_noncreature_nonkindred_artifact() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for types in [
        vec![CardType::Artifact, CardType::Creature],
        vec![CardType::Artifact],
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Subtype recipient",
        )
        .card_types(types.clone())
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::AddSubtypes(vec![Subtype::Goblin]),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert_eq!(
            game.current_subtypes(source)
                .unwrap()
                .contains(&Subtype::Goblin),
            types.contains(&CardType::Creature),
            "CR 205.3d: Goblin requires creature or kindred card type"
        );
    }
}

#[test]
fn cr_205_1a_type_replacement_prunes_subtypes_but_retains_supertypes_and_counters() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, CounterType, Subtype, Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition =
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Type recipient")
            .card_types(vec![CardType::Land, CardType::Creature])
            .subtypes(vec![Subtype::Forest, Subtype::Dryad])
            .supertypes(vec![Supertype::Legendary])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(source, CounterType::PlusOnePlusOne, 2);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::SetCardTypes(vec![CardType::Artifact]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert_eq!(
        game.current_card_types(source),
        Some(vec![CardType::Artifact])
    );
    assert!(game.current_subtypes(source).unwrap().is_empty());
    assert_eq!(
        game.current_supertypes(source),
        Some(vec![Supertype::Legendary])
    );
    assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 2);
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(
        game.current_card_types(source)
            .unwrap()
            .contains(&CardType::Land)
    );
    assert!(
        game.current_card_types(source)
            .unwrap()
            .contains(&CardType::Creature)
    );
    assert!(
        game.current_subtypes(source)
            .unwrap()
            .contains(&Subtype::Forest)
    );
    assert!(
        game.current_subtypes(source)
            .unwrap()
            .contains(&Subtype::Dryad)
    );
}

#[test]
fn cr_205_1a_creature_subtypes_survive_type_loss_only_if_kindred_remains() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for kindred in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut types = vec![CardType::Creature, CardType::Enchantment];
        if kindred {
            types.push(CardType::Kindred);
        }
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Shared subtype family",
        )
        .card_types(types)
        .subtypes(vec![Subtype::Goblin])
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::RemoveCardTypes(vec![CardType::Creature]),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert!(
            !game
                .current_card_types(source)
                .unwrap()
                .contains(&CardType::Creature)
        );
        assert_eq!(
            game.current_subtypes(source)
                .unwrap()
                .contains(&Subtype::Goblin),
            kindred,
            "CR 205.1a: Goblin remains supported by the kindred type"
        );
    }
}

#[test]
fn cr_205_1a_setting_subtypes_replaces_only_the_corresponding_family() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Multiple subtype families",
    )
    .card_types(vec![CardType::Land, CardType::Creature])
    .subtypes(vec![Subtype::Forest, Subtype::Dryad])
    .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (replacement, present, absent) in [
        (
            Subtype::Goblin,
            vec![Subtype::Forest, Subtype::Goblin],
            vec![Subtype::Dryad],
        ),
        (
            Subtype::Island,
            vec![Subtype::Island, Subtype::Goblin],
            vec![Subtype::Forest, Subtype::Dryad],
        ),
    ] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::SetSubtypes(vec![replacement]),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        let subtypes = game.current_subtypes(source).unwrap();
        for subtype in present {
            assert!(subtypes.contains(&subtype));
        }
        for subtype in absent {
            assert!(!subtypes.contains(&subtype));
        }
        assert_eq!(
            game.current_card_types(source),
            Some(vec![CardType::Land, CardType::Creature])
        );
    }
}

#[test]
fn cr_205_3d_setting_a_land_subtype_does_not_give_it_to_a_nonland() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for kind in [CardType::Land, CardType::Artifact] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Land subtype recipient",
        )
        .card_types(vec![kind])
        .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::SetSubtypes(vec![Subtype::Island]),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        assert_eq!(
            game.current_subtypes(source)
                .unwrap()
                .contains(&Subtype::Island),
            kind == CardType::Land,
            "CR 205.3d: setting Island cannot grant a land type to a nonland"
        );
    }
}

#[test]
fn cr_205_4b_supertype_changes_leave_types_subtypes_and_other_supertypes_unchanged() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Subtype, Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Supertype recipient",
    )
    .card_types(vec![CardType::Land])
    .subtypes(vec![Subtype::Forest])
    .supertypes(vec![Supertype::Basic, Supertype::Snow])
    .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for (modification, expected) in [
        (
            Modification::AddSupertypes(vec![Supertype::Legendary]),
            vec![Supertype::Basic, Supertype::Snow, Supertype::Legendary],
        ),
        (
            Modification::RemoveSupertypes(vec![Supertype::Basic]),
            vec![Supertype::Snow, Supertype::Legendary],
        ),
    ] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            modification,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        let actual = game.current_supertypes(source).unwrap();
        assert_eq!(actual.len(), expected.len());
        for supertype in expected {
            assert!(actual.contains(&supertype));
        }
        assert_eq!(game.current_card_types(source), Some(vec![CardType::Land]));
        assert_eq!(game.current_subtypes(source), Some(vec![Subtype::Forest]));
    }
}

#[test]
fn cr_205_4c_and_g_basic_and_snow_filters_follow_supertypes_not_name_or_land_type() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, Subtype, Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for remove_basic in [false, true] {
        for remove_snow in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Snow-Covered Forest",
            )
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .supertypes(vec![Supertype::Basic, Supertype::Snow])
            .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let mut removed = vec![];
            if remove_basic {
                removed.push(Supertype::Basic);
            }
            if remove_snow {
                removed.push(Supertype::Snow);
            }
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::RemoveSupertypes(removed),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            assert_eq!(
                game.current_name(source).as_deref(),
                Some("Snow-Covered Forest")
            );
            assert!(
                game.current_subtypes(source)
                    .unwrap()
                    .contains(&Subtype::Forest)
            );
            let ctx = EffectContext::new_default(source, alice);
            for (filter, expected) in [
                (
                    ObjectFilter::land().with_supertype(Supertype::Basic),
                    !remove_basic,
                ),
                (ObjectFilter::land().nonbasic(), remove_basic),
                (
                    ObjectFilter::land().with_supertype(Supertype::Snow),
                    !remove_snow,
                ),
                (
                    ObjectFilter::land().without_supertype(Supertype::Snow),
                    remove_snow,
                ),
            ] {
                assert_eq!(
                    ironsmith::effects::helpers::resolve_objects_from_spec(
                        &game,
                        &ChooseSpec::All(filter),
                        &ctx
                    )
                    .unwrap()
                    .contains(&source),
                    expected
                );
            }
        }
    }
}

#[test]
fn cr_704_5k_world_rule_keeps_unique_newest_and_removes_all_on_a_tie() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for simultaneous in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ids = vec![];
        for (index, owner) in [alice, bob, alice].into_iter().enumerate() {
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                format!("World {index}"),
            )
            .card_types(vec![CardType::Enchantment])
            .supertypes(if simultaneous {
                vec![]
            } else {
                vec![Supertype::World]
            })
            .build();
            ids.push(game.create_object_from_definition(&definition, owner, Zone::Battlefield));
        }
        if simultaneous {
            ApplyContinuousEffect::new(
                EffectTarget::AllPermanents,
                Modification::AddSupertypes(vec![Supertype::World]),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(ids[0], alice))
            .unwrap();
        }
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        for (index, id) in ids.into_iter().enumerate() {
            assert_eq!(
                game.battlefield.contains(&id),
                !simultaneous && index == 2,
                "CR 704.5k: simultaneous={simultaneous}, object={index}"
            );
        }
        let graveyard_total =
            game.player(alice).unwrap().graveyard.len() + game.player(bob).unwrap().graveyard.len();
        assert_eq!(graveyard_total, if simultaneous { 3 } else { 2 });
    }
}

#[test]
fn cr_201_2a_nameless_legendary_permanents_do_not_violate_the_legend_rule() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::rules::state_based::StateBasedAction;
    use ironsmith::{Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for nameless in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let first = counter_creature(&mut game, Zone::Battlefield);
        let second = counter_creature(&mut game, Zone::Battlefield);
        if nameless {
            assert!(game.set_face_down(first));
            assert!(game.set_face_down(second));
        }
        ApplyContinuousEffect::new(
            EffectTarget::AllPermanents,
            Modification::AddSupertypes(vec![Supertype::Legendary]),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(first, alice))
        .unwrap();
        for id in [first, second] {
            assert!(
                game.current_supertypes(id)
                    .unwrap()
                    .contains(&Supertype::Legendary)
            );
        }
        assert_eq!(
            ironsmith::check_state_based_actions(&game)
                .iter()
                .any(|action| matches!(action, StateBasedAction::LegendRuleViolation { .. })),
            !nameless,
            "CR 201.2a / 704.5j: nameless legendary permanents do not share a name"
        );
    }
}

#[test]
fn cr_205_4e_legendary_spell_requires_own_unphased_legendary_creature_or_planeswalker() {
    use ironsmith::{CardId, CardType, Effect, LegalAction, ManaCost, Phase, Supertype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for spell_type in [CardType::Instant, CardType::Sorcery] {
        for (support_type, legendary, owner, phased, allowed) in [
            (CardType::Artifact, true, alice, false, false),
            (CardType::Land, true, alice, false, false),
            (CardType::Creature, false, alice, false, false),
            (CardType::Creature, true, bob, false, false),
            (CardType::Creature, true, alice, true, false),
            (CardType::Creature, true, alice, false, true),
            (CardType::Planeswalker, true, alice, false, true),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let support = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Legendary support",
            )
            .card_types(vec![support_type])
            .supertypes(if legendary {
                vec![Supertype::Legendary]
            } else {
                vec![]
            })
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .loyalty(3)
            .build();
            let support = game.create_object_from_definition(&support, owner, Zone::Battlefield);
            if phased {
                game.phase_out(support);
            }
            let spell = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Legendary spell",
            )
            .card_types(vec![spell_type])
            .supertypes(vec![Supertype::Legendary])
            .mana_cost(ManaCost::from_symbols(vec![]))
            .with_spell_effect(vec![Effect::gain_life(1)])
            .build();
            let spell = game.create_object_from_definition(&spell, alice, Zone::Hand);
            assert_eq!(ironsmith::compute_legal_actions(&game,alice).unwrap().iter().any(|action|
                matches!(action,LegalAction::CastSpell {spell_id,..} if *spell_id==spell)),allowed,
                "CR 205.4e: spell={spell_type:?}, support={support_type:?}, legendary={legendary}, owner={owner:?}, phased={phased}");
        }
    }
}

#[test]
fn cr_205_4h_ongoing_schemes_are_exempt_from_normal_scheme_cleanup() {
    use ironsmith::{ArchenemyVariant, CardId, CardType, StateBasedAction, Supertype};
    let alice = PlayerId::from_index(0);
    for ongoing in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let deck = (0..20)
            .map(|i| {
                ironsmith::cards::builders::CardDefinitionBuilder::new(
                    CardId::new(),
                    format!("Audit scheme {i}"),
                )
                .card_types(vec![CardType::Scheme])
                .supertypes(if ongoing {
                    vec![Supertype::Ongoing]
                } else {
                    vec![]
                })
                .build()
            })
            .collect();
        game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, deck)])
            .unwrap();
        let scheme = game.set_scheme_in_motion(alice).unwrap();
        let stable = game.object(scheme).unwrap().stable_id;
        assert!(game.is_face_up_scheme(scheme));
        // These fixtures have no abilities. Model the point after their
        // set-in-motion event has been dispatched and produced no triggers.
        game.effect_store.pending_trigger_events.clear();
        assert!(game.stack.is_empty());
        assert!(game.effect_store.pending_trigger_entries.is_empty());
        assert_eq!(
            ironsmith::check_state_based_actions(&game)
                .iter()
                .any(|action| matches!(action,StateBasedAction::RecycleScheme(id) if *id==scheme)),
            !ongoing
        );
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert_eq!(game.is_face_up_scheme(scheme), ongoing);
        assert_eq!(
            game.scheme_deck(alice).unwrap().len(),
            if ongoing { 19 } else { 20 }
        );
        if !ongoing {
            let bottom = game.scheme_deck(alice).unwrap()[0];
            assert_eq!(game.object(bottom).unwrap().stable_id, stable);
            assert!(!game.is_face_up_scheme(bottom));
        }
    }
}

#[test]
fn cr_704_6e_scheme_cleanup_waits_for_trigger_of_abandoned_scheme() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    use ironsmith::game_state::StackEntry;
    use ironsmith::triggers::{Trigger, TriggerEvent};
    use ironsmith::{
        Ability, ArchenemyVariant, CardId, CardType, Effect, ObjectSnapshot, PlayerFilter,
        StateBasedAction, Supertype,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let deck = (0..20)
        .map(|i| {
            CardDefinitionBuilder::new(CardId::new(), format!("Pending scheme {i}"))
                .card_types(vec![CardType::Scheme])
                .supertypes(if i % 2 == 0 {
                    vec![Supertype::Ongoing]
                } else {
                    vec![]
                })
                .with_ability(Ability::triggered(
                    Trigger::keyword_action_from_source(
                        KeywordActionKind::SetSchemeInMotion,
                        PlayerFilter::You,
                    ),
                    vec![Effect::gain_life(1)],
                ))
                .build()
        })
        .collect();
    game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, deck)])
        .unwrap();
    let mut ongoing = None;
    let mut ordinary = None;
    while ongoing.is_none() || ordinary.is_none() {
        let id = game.set_scheme_in_motion(alice).unwrap();
        if game.scheme_is_ongoing(id) {
            ongoing = Some(id);
        } else {
            ordinary = Some(id);
        }
    }
    let ongoing = ongoing.unwrap();
    let ordinary = ordinary.unwrap();
    // Model a dispatched set-in-motion trigger on the stack. Other triggers
    // have resolved. An effect can abandon its source while this trigger waits.
    game.effect_store.pending_trigger_events.clear();
    let snapshot = ObjectSnapshot::from_object(game.object(ongoing).unwrap(), &game);
    let event = TriggerEvent::new_with_provenance(
        KeywordActionEvent::new(KeywordActionKind::SetSchemeInMotion, alice, ongoing, 1),
        Default::default(),
    );
    game.stack.push(
        StackEntry::ability(ongoing, alice, vec![Effect::gain_life(1)])
            .with_triggering_event(event)
            .with_source_snapshot(snapshot),
    );
    assert!(
        !ironsmith::check_state_based_actions(&game)
            .contains(&StateBasedAction::RecycleScheme(ordinary)),
        "Control: the trigger blocks cleanup while its source is face up"
    );
    game.abandon_scheme(ongoing).unwrap();
    game.effect_store.pending_trigger_events.clear();
    assert!(!game.is_face_up_scheme(ongoing));
    assert_eq!(game.stack.len(), 1);
    let premature = ironsmith::check_state_based_actions(&game)
        .contains(&StateBasedAction::RecycleScheme(ordinary));
    game.stack.clear();
    assert!(
        ironsmith::check_state_based_actions(&game)
            .contains(&StateBasedAction::RecycleScheme(ordinary)),
        "Control: cleanup is allowed once the last scheme trigger leaves the stack"
    );
    assert!(
        !premature,
        "CR 704.6e: a triggered ability of any scheme blocks cleanup, even after its source was abandoned"
    );
}

#[test]
fn cr_208_3_noncreature_vehicle_has_printed_pt_only_outside_battlefield() {
    use ironsmith::{CardId, CardType, PowerToughness, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
        Zone::Battlefield,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let vehicle = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Uncrewed audit vehicle",
        )
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(3, 4))
        .build();
        let id = game.create_object_from_definition(&vehicle, alice, zone);
        let expected = if zone == Zone::Battlefield {
            (None, None)
        } else {
            (Some(3), Some(4))
        };
        assert_eq!(
            (game.current_power(id), game.current_toughness(id)),
            expected,
            "CR 208.3: noncreature vehicle in {zone:?}"
        );
    }
}

#[test]
fn cr_208_5_creature_without_printed_pt_has_zero_power_and_toughness() {
    use ironsmith::{CardId, CardType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    // A noncreature artifact becomes a creature without a P/T-setting effect.
    // This models a type-changing effect applied to an artifact lacking P/T.
    let artifact = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Audit artifact without numbers",
    )
    .card_types(vec![CardType::Artifact])
    .build();
    let id = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    assert_eq!(
        (game.current_power(id), game.current_toughness(id)),
        (None, None)
    );
    let mut ctx = EffectContext::new_default(id, alice);
    ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(id),
        ironsmith::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        ironsmith::Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(
        game.current_card_types(id)
            .unwrap()
            .contains(&CardType::Creature)
    );
    assert_eq!(
        (game.current_power(id), game.current_toughness(id)),
        (Some(0), Some(0)),
        "CR 208.5: a creature with no power/toughness values uses zero"
    );
}

#[test]
fn cr_704_5f_creature_with_missing_toughness_dies_as_zero_toughness() {
    use ironsmith::{CardId, CardType, StateBasedAction, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let artifact = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Zero toughness audit artifact",
    )
    .card_types(vec![CardType::Artifact])
    .build();
    let id = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(id, alice);
    ironsmith::effects::ApplyContinuousEffect::new(
        ironsmith::continuous::EffectTarget::Specific(id),
        ironsmith::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
        ironsmith::Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(
        game.current_card_types(id)
            .unwrap()
            .contains(&CardType::Creature)
    );
    assert!(
        ironsmith::check_state_based_actions(&game).contains(&StateBasedAction::ObjectDies(id)),
        "CR 208.5 / 704.5f: missing toughness is zero, so the creature must die"
    );
    ironsmith::apply_state_based_actions(&mut game).unwrap();
    assert!(!game.battlefield.contains(&id));
}

#[test]
fn cr_208_4_base_pt_excludes_modifiers_counters_and_switching() {
    use ironsmith::continuous::{EffectTarget, Modification, PtSublayer};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CounterType, Until, Zone};
    let alice = PlayerId::from_index(0);
    for reverse_timestamps in [false, true] {
        for switches in 0..=2 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let id = counter_creature(&mut game, Zone::Battlefield);
            let mut ctx = EffectContext::new_default(id, alice);
            let mut modifications = vec![
                Modification::SetPowerToughness {
                    power: 2.into(),
                    toughness: 5.into(),
                    sublayer: PtSublayer::Setting,
                },
                Modification::ModifyPowerToughness {
                    power: 3,
                    toughness: -1,
                },
            ];
            for _ in 0..switches {
                modifications.push(Modification::SwitchPowerToughness);
            }
            if reverse_timestamps {
                modifications.reverse();
            }
            for modification in modifications {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(id),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut ctx)
                .unwrap();
            }
            game.add_counters(id, CounterType::PlusOnePlusOne, 2);
            let chars = game.current_characteristics(id).unwrap();
            assert_eq!(
                (chars.base_power, chars.base_toughness),
                (Some(2), Some(5)),
                "CR 208.4b: reverse={reverse_timestamps}, switches={switches}"
            );
            let expected = if switches % 2 == 0 {
                (Some(7), Some(6))
            } else {
                (Some(6), Some(7))
            };
            assert_eq!(
                (chars.power, chars.toughness),
                expected,
                "CR 208.4a / 613.4b-d: sublayers apply regardless of timestamps between sublayers"
            );
        }
    }
}

#[test]
fn cr_208_3a_pt_modifier_created_before_animation_applies_when_creature() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, PowerToughness, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let vehicle = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Vehicle awaiting animation",
    )
    .card_types(vec![CardType::Artifact])
    .subtypes(vec![Subtype::Vehicle])
    .power_toughness(PowerToughness::fixed(3, 4))
    .build();
    let id = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(id, alice);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::ModifyPowerToughness {
            power: 1,
            toughness: 2,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(
        !game
            .current_card_types(id)
            .unwrap()
            .contains(&CardType::Creature)
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(id),
        Modification::AddCardTypes(vec![CardType::Creature]),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(
        (game.current_power(id), game.current_toughness(id)),
        (Some(4), Some(6)),
        "CR 208.3a: a modifier created before animation must still apply after animation"
    );
}

#[test]
fn cr_208_2a_pt_cda_functions_in_every_zone_and_uses_zero_for_unchosen_player() {
    use ironsmith::{
        Ability, CardId, CardType, ObjectFilter, PowerToughness, StaticAbility, Value, Zone,
    };
    let alice = PlayerId::from_index(0);
    for unknown_player in [false, true] {
        for zone in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Command,
            Zone::Battlefield,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            counter_creature(&mut game, Zone::Battlefield);
            counter_creature(&mut game, Zone::Battlefield);
            let count = Value::Count(ObjectFilter::creature().controlled_by(if unknown_player {
                PlayerFilter::ChosenPlayer
            } else {
                PlayerFilter::You
            }));
            let value = Value::Add(Box::new(Value::Fixed(1)), Box::new(count));
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "CDA audit creature",
            )
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 0))
            .with_ability(Ability::static_ability(
                StaticAbility::characteristic_defining_pt(value.clone(), value),
            ))
            .build();
            let id = game.create_object_from_definition(&definition, alice, zone);
            let expected = if unknown_player {
                1
            } else if zone == Zone::Battlefield {
                4
            } else {
                3
            };
            assert_eq!(
                (game.current_power(id), game.current_toughness(id)),
                (Some(expected), Some(expected)),
                "CR 208.2a: zone={zone:?}, undetermined chosen player={unknown_player}"
            );
        }
    }
}

#[test]
fn cr_208_2b_entry_chosen_pt_is_copiable_and_resets_outside_battlefield() {
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::RuntimeModification;
    use ironsmith::effects::{ApplyContinuousEffect, MoveToZoneEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, PowerToughness, StaticAbility, Until, Zone,
    };
    struct PickPt(usize);
    impl DecisionMaker for PickPt {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.options.len(), 2);
            vec![self.0]
        }
    }
    let alice = PlayerId::from_index(0);
    for (choice, expected) in [(0, (Some(5), Some(1))), (1, (Some(1), Some(5)))] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Entry choice creature")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(0,0))
            .with_ability(Ability::static_ability(StaticAbility::choose_power_toughness_as_enters_or_turns_face_up(
                vec![(5,1),(1,5)], "As this creature enters or is turned face up, it becomes your choice of 5/1 or 1/5.".into())))
            .build();
        let card = game.create_object_from_definition(&def, alice, Zone::Hand);
        assert_eq!(
            (game.current_power(card), game.current_toughness(card)),
            (Some(0), Some(0))
        );
        let stable = game.object(card).unwrap().stable_id;
        let mut dm = PickPt(choice);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
            .execute(
                &mut game,
                &mut EffectContext::new_default(card, alice).with_decision_maker(&mut dm),
            )
            .unwrap();
        let entered = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().stable_id == stable)
            .unwrap();
        assert_eq!(
            (game.current_power(entered), game.current_toughness(entered)),
            expected
        );
        let copy = counter_creature(&mut game, Zone::Battlefield);
        ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(copy),
            RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(entered),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: vec![],
                copy_exception_surface: None,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(copy, alice))
        .unwrap();
        assert_eq!(
            (game.current_power(copy), game.current_toughness(copy)),
            expected,
            "CR 208.2b: entry-selected power/toughness is copiable"
        );
        assert!(game.set_face_down(entered));
        assert_eq!(
            (game.current_power(entered), game.current_toughness(entered)),
            (Some(2), Some(2))
        );
        let mut dm = PickPt(1 - choice);
        ironsmith::effects::TurnFaceUpEffect::new(ChooseSpec::SpecificObject(entered))
            .execute(
                &mut game,
                &mut EffectContext::new_default(entered, alice).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(
            (game.current_power(entered), game.current_toughness(entered)),
            (expected.1, expected.0),
            "CR 208.2b: turning face up applies a fresh characteristic choice"
        );
        let gone = game
            .move_object_by_effect(entered, Zone::Graveyard)
            .unwrap();
        assert_eq!(
            (game.current_power(gone), game.current_toughness(gone)),
            (Some(0), Some(0)),
            "CR 208.2b: outside the battlefield these values are zero"
        );
    }
}

#[test]
fn cr_210_1_battle_printed_defense_and_entry_counters_follow_zone_changes() {
    use ironsmith::{CardId, CardType, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    for printed in [1, 4, 7] {
        for origin in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Command,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Defense audit battle",
            )
            .card_types(vec![CardType::Battle])
            .defense(printed)
            .build();
            let id = game.create_object_from_definition(&def, alice, origin);
            assert_eq!(game.object(id).unwrap().defense(), Some(printed));
            assert_eq!(game.counter_count(id, CounterType::Defense), 0);
            let stable = game.object(id).unwrap().stable_id;
            ironsmith::effects::MoveToZoneEffect::new(
                ironsmith::ChooseSpec::SpecificObject(id),
                Zone::Battlefield,
                false,
            )
            .execute(&mut game, &mut EffectContext::new_default(id, alice))
            .unwrap();
            let entered = *game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().stable_id == stable)
                .unwrap();
            assert_eq!(
                game.counter_count(entered, CounterType::Defense),
                printed,
                "CR 310.4b: battle enters with printed defense counters; origin={origin:?}"
            );
            game.remove_counters(entered, CounterType::Defense, printed, None, None);
            assert_eq!(game.counter_count(entered, CounterType::Defense), 0);
            let departed = game.move_object_by_effect(entered, Zone::Exile).unwrap();
            assert_eq!(
                game.object(departed).unwrap().defense(),
                Some(printed),
                "CR 310.4a: outside battlefield defense is printed, even after counters were depleted"
            );
            assert_eq!(game.counter_count(departed, CounterType::Defense), 0);
            ironsmith::effects::MoveToZoneEffect::new(
                ironsmith::ChooseSpec::SpecificObject(departed),
                Zone::Battlefield,
                false,
            )
            .execute(&mut game, &mut EffectContext::new_default(departed, alice))
            .unwrap();
            let returned = *game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().stable_id == stable)
                .unwrap();
            assert_eq!(
                game.counter_count(returned, CounterType::Defense),
                printed,
                "CR 310.4b: a new entry restores the printed number of counters"
            );
        }
    }
}

#[test]
fn cr_211_1_and_212_1_vanguard_modifiers_apply_to_each_owners_initial_totals() {
    use ironsmith::{Ability, CardId, CardType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (hand, life) in [(-2, -5), (0, 0), (3, 8)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut cards = Vec::new();
        for (owner, h, l) in [(alice, hand, life), (bob, -hand, -life)] {
            let mut def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                format!("Vanguard {owner:?}"),
            )
            .card_types(vec![CardType::Vanguard])
            .build();
            def.card.hand_modifier = h;
            def.card.life_modifier = l;
            cards.push((owner, def));
        }
        game.enable_vanguard(cards).unwrap();
        game.refresh_continuous_state().unwrap();
        for (owner, h, l) in [(alice, hand, life), (bob, -hand, -life)] {
            let player = game.player(owner).unwrap();
            assert_eq!(player.starting_life, 20 + l);
            assert_eq!(player.life, 20 + l);
            assert_eq!(game.vanguard_starting_hand_size(owner), (7 + h) as usize);
            assert_eq!(player.max_hand_size, 7 + h);
            let avatar = game.vanguard_card(owner).unwrap();
            assert_eq!(game.object(avatar).unwrap().zone, Zone::Command);
            assert_eq!(game.object(avatar).unwrap().owner, owner);
            assert_eq!(game.current_controller(avatar), Some(owner));
        }
        // Hand-size effects add to the owner's Vanguard-adjusted baseline.
        let grant = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Hand size grant",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::increase_maximum_hand_size(PlayerFilter::You, 2),
        ))
        .build();
        let grant = game.create_object_from_definition(&grant, alice, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(alice).unwrap().max_hand_size, 9 + hand);
        assert_eq!(game.player(bob).unwrap().max_hand_size, 7 - hand);
        game.move_object_by_effect(grant, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(alice).unwrap().max_hand_size, 7 + hand);
    }
}

#[test]
fn cr_313_2_and_4_vanguard_stays_in_command_and_static_ability_functions() {
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let cards = [alice, bob]
        .into_iter()
        .map(|owner| {
            let mut builder = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                format!("Command vanguard {owner:?}"),
            )
            .card_types(vec![CardType::Vanguard]);
            if owner == alice {
                builder = builder.with_ability(Ability::static_ability(
                    StaticAbility::increase_maximum_hand_size(PlayerFilter::You, 2),
                ));
            }
            (owner, builder.build())
        })
        .collect();
    game.enable_vanguard(cards).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.player(alice).unwrap().max_hand_size, 9);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 7);
    let avatar = game.vanguard_card(alice).unwrap();
    for destination in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Battlefield,
    ] {
        ironsmith::effects::MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(avatar),
            destination,
            false,
        )
        .execute(&mut game, &mut EffectContext::new_default(avatar, alice))
        .unwrap();
        assert_eq!(
            game.object(avatar).map(|o| o.zone),
            Some(Zone::Command),
            "CR 313.2: attempted move to {destination:?} must leave the Vanguard in command"
        );
        assert!(!game.battlefield.contains(&avatar));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(alice).unwrap().max_hand_size, 9);
    }
}

#[test]
fn cr_300_2a_land_with_another_type_can_only_be_played_as_land() {
    use ironsmith::{CardId, CardType, Effect, LegalAction, ManaCost, Phase, Zone};
    let alice = PlayerId::from_index(0);
    for other in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Kindred,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let card = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Multitype audit land",
        )
        .card_types(vec![CardType::Land, other])
        .mana_cost(ManaCost::from_symbols(vec![]))
        .with_spell_effect(vec![Effect::gain_life(1)])
        .build();
        let id = game.create_object_from_definition(&card, alice, Zone::Hand);
        let actions = ironsmith::compute_legal_actions(&game, alice).unwrap();
        assert!(
            actions
                .iter()
                .any(|a| matches!(a,LegalAction::PlayLand {land_id} if *land_id==id)),
            "CR 300.2a: land/{other:?} can be played as a land"
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a,LegalAction::CastSpell {spell_id,..} if *spell_id==id)),
            "CR 300.2a: land/{other:?} cannot be cast, even with a payable mana cost"
        );
    }
}

#[test]
fn cr_300_2b_kindred_spells_obey_timing_of_their_other_card_type() {
    use ironsmith::{CardId, CardType, Effect, LegalAction, ManaCost, Phase, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for kindred in [false, true] {
        for other in [
            CardType::Instant,
            CardType::Sorcery,
            CardType::Artifact,
            CardType::Enchantment,
            CardType::Creature,
        ] {
            for active in [alice, bob] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = active;
                game.turn.priority_player = Some(alice);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                let mut types = vec![other];
                if kindred {
                    types.push(CardType::Kindred);
                }
                let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    CardId::new(),
                    "Kindred timing subject",
                )
                .card_types(types)
                .mana_cost(ManaCost::from_symbols(vec![]))
                .with_spell_effect(vec![Effect::gain_life(1)])
                .build();
                let id = game.create_object_from_definition(&def, alice, Zone::Hand);
                let legal = ironsmith::compute_legal_actions(&game, alice)
                    .unwrap()
                    .iter()
                    .any(|a| matches!(a,LegalAction::CastSpell {spell_id,..} if *spell_id==id));
                assert_eq!(
                    legal,
                    active == alice || other == CardType::Instant,
                    "CR 300.2b: kindred={kindred}, other={other:?}, active={active:?}"
                );
            }
        }
    }
}

#[test]
fn cr_301_5c_equipment_detaches_after_type_changes_but_stays_on_battlefield() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, StateBasedAction, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for change in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = counter_creature(&mut game, Zone::Battlefield);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Audit Equipment",
        )
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
        let equipment = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
        assert!(
            !ironsmith::check_state_based_actions(&game)
                .contains(&StateBasedAction::AttachmentBecomesUnattached(equipment))
        );
        let (target, modification) = match change {
            0 => (
                host,
                Modification::RemoveCardTypes(vec![CardType::Creature]),
            ),
            1 => (
                equipment,
                Modification::RemoveSubtypes(vec![Subtype::Equipment]),
            ),
            2 => (
                equipment,
                Modification::AddCardTypes(vec![CardType::Creature]),
            ),
            _ => (
                host,
                Modification::ChangeController(PlayerId::from_index(1)),
            ),
        };
        ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            modification,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(equipment, alice))
        .unwrap();
        let should_detach = change != 3;
        assert_eq!(
            ironsmith::check_state_based_actions(&game)
                .contains(&StateBasedAction::AttachmentBecomesUnattached(equipment)),
            should_detach,
            "CR 301.5c/d: change={change}"
        );
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(game.battlefield.contains(&equipment));
        assert_eq!(
            game.object(equipment).unwrap().attached_to,
            if should_detach {
                None
            } else {
                Some(AttachmentTarget::Object(host))
            }
        );
        assert_eq!(game.current_controller(equipment), Some(alice));
    }
}

#[test]
fn cr_301_5b_illegal_attachment_preserves_previous_equipped_creature() {
    use ironsmith::effects::{AttachObjectsEffect, AttachToEffect};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for plural_path in [false, true] {
        for destination in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let old = counter_creature(&mut game, Zone::Battlefield);
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Moving Equipment",
            )
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
            let equipment = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(old)));
            let target = match destination {
                0 => equipment,
                1 => game.create_object_from_definition(&def, alice, Zone::Battlefield),
                2 => counter_creature(&mut game, Zone::Graveyard),
                _ => {
                    let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(
                        CardId::new(),
                        "Opponent host",
                    )
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
                    .build();
                    game.create_object_from_definition(&creature, bob, Zone::Battlefield)
                }
            };
            let mut ctx = EffectContext::new_default(equipment, alice);
            if plural_path {
                let _ = AttachObjectsEffect::new(
                    ChooseSpec::SpecificObject(equipment),
                    ChooseSpec::SpecificObject(target),
                )
                .execute(&mut game, &mut ctx);
            } else {
                let _ = AttachToEffect::new(ChooseSpec::SpecificObject(target))
                    .execute(&mut game, &mut ctx);
            }
            let expected = if destination == 3 { target } else { old };
            assert_eq!(
                game.object(equipment).unwrap().attached_to,
                Some(AttachmentTarget::Object(expected)),
                "CR 301.5b/c/d: destination={destination}, plural_path={plural_path}"
            );
            assert!(
                game.object(expected)
                    .unwrap()
                    .attachments
                    .contains(&equipment)
            );
            if destination == 3 {
                assert!(!game.object(old).unwrap().attachments.contains(&equipment));
                assert_eq!(game.current_controller(equipment), Some(alice));
            }
        }
    }
}

#[test]
fn cr_301_6_fortification_uses_land_hosts_and_detaches_when_illegal() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, AttachToEffect};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, StateBasedAction, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for change in 0..5 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let land =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Fortified land")
                .card_types(vec![CardType::Land])
                .build();
        let land = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Audit Fortification",
        )
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Fortification])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
        let fort = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let creature = counter_creature(&mut game, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(fort, alice);
        AttachToEffect::new(ChooseSpec::SpecificObject(land))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.object(fort).unwrap().attached_to,
            Some(AttachmentTarget::Object(land))
        );
        let _ =
            AttachToEffect::new(ChooseSpec::SpecificObject(creature)).execute(&mut game, &mut ctx);
        assert_eq!(
            game.object(fort).unwrap().attached_to,
            Some(AttachmentTarget::Object(land)),
            "CR 301.6: a nonland creature cannot be fortified"
        );
        let (target, modification, detach) = match change {
            0 => (
                land,
                Modification::RemoveCardTypes(vec![CardType::Land]),
                true,
            ),
            1 => (
                fort,
                Modification::RemoveSubtypes(vec![Subtype::Fortification]),
                true,
            ),
            2 => (
                fort,
                Modification::AddCardTypes(vec![CardType::Creature]),
                true,
            ),
            3 => (
                fort,
                Modification::AddCardTypes(vec![CardType::Land]),
                false,
            ),
            _ => (
                land,
                Modification::ChangeController(PlayerId::from_index(1)),
                false,
            ),
        };
        ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            modification,
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            ironsmith::check_state_based_actions(&game)
                .contains(&StateBasedAction::AttachmentBecomesUnattached(fort)),
            detach,
            "CR 301.6: change={change}"
        );
        ironsmith::apply_state_based_actions(&mut game).unwrap();
        assert!(game.battlefield.contains(&fort));
        assert_eq!(
            game.object(fort).unwrap().attached_to,
            if detach {
                None
            } else {
                Some(AttachmentTarget::Object(land))
            }
        );
        assert_eq!(game.current_controller(fort), Some(alice));
    }
}

#[test]
fn cr_704_5f_g_h_damage_sbas_use_current_toughness_and_destruction_rules() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, PowerToughness, StateBasedAction, StaticAbility,
        Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    for deathtouch in [false, true] {
        for indestructible in [false, true] {
            for damage in [0, 3, 4, 5] {
                for delta in [-4, -1, 0, 2] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let mut source = ironsmith::cards::builders::CardDefinitionBuilder::new(
                        CardId::new(),
                        "Damage audit source",
                    )
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(3, 4));
                    if deathtouch {
                        source = source
                            .with_ability(Ability::static_ability(StaticAbility::deathtouch()));
                    }
                    let source = game.create_object_from_definition(
                        &source.build(),
                        alice,
                        Zone::Battlefield,
                    );
                    let mut target = ironsmith::cards::builders::CardDefinitionBuilder::new(
                        CardId::new(),
                        "Damage audit recipient",
                    )
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(3, 4));
                    if indestructible {
                        target = target
                            .with_ability(Ability::static_ability(StaticAbility::indestructible()));
                    }
                    let target = game.create_object_from_definition(
                        &target.build(),
                        alice,
                        Zone::Battlefield,
                    );
                    let mut ctx = EffectContext::new_default(source, alice);
                    DealDamageEffect::new(damage, ChooseSpec::SpecificObject(target))
                        .execute(&mut game, &mut ctx)
                        .unwrap();
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(target),
                        Modification::ModifyToughness(delta),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                    assert_eq!(game.damage_on(target), damage as u32);
                    let toughness = 4 + delta;
                    let dies = toughness <= 0
                        || (!indestructible && damage > 0 && (damage >= toughness || deathtouch));
                    assert_eq!(
                        ironsmith::check_state_based_actions(&game)
                            .contains(&StateBasedAction::ObjectDies(target)),
                        dies,
                        "CR 704.5f-h: damage={damage}, current toughness={toughness}, deathtouch={deathtouch}, indestructible={indestructible}"
                    );
                    ironsmith::apply_state_based_actions(&mut game).unwrap();
                    assert_eq!(game.battlefield.contains(&target), !dies);
                }
            }
        }
    }
}

#[test]
fn cr_704_5h_old_deathtouch_damage_does_not_destroy_after_indestructible_is_lost() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, PowerToughness, StateBasedAction, StaticAbility,
        Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let build = |name, ability| {
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(ability))
            .build()
    };
    let source = game.create_object_from_definition(
        &build("Deathtouch source", StaticAbility::deathtouch()),
        alice,
        Zone::Battlefield,
    );
    let target = game.create_object_from_definition(
        &build("Indestructible target", StaticAbility::indestructible()),
        alice,
        Zone::Battlefield,
    );
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(target))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(game.battlefield.contains(&target));
    assert_eq!(game.damage_on(target), 1);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(target),
        Modification::RemoveAllAbilities,
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    assert!(!game.object_has_ability(target, &StaticAbility::indestructible()));
    assert!(
        !ironsmith::check_state_based_actions(&game)
            .contains(&StateBasedAction::ObjectDies(target)),
        "CR 704.5h: only deathtouch damage since the last SBA check matters"
    );
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(target))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    assert!(
        ironsmith::check_state_based_actions(&game).contains(&StateBasedAction::ObjectDies(target)),
        "Control: fresh deathtouch damage must destroy the now-destructible creature"
    );
}

#[test]
fn cr_704_5f_g_h_regeneration_replaces_damage_destruction_but_not_zero_toughness() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect, RegenerateEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, PowerToughness, StaticAbility, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    for mode in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Regeneration damage source",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4));
        if mode == 1 {
            def = def.with_ability(Ability::static_ability(StaticAbility::deathtouch()));
        }
        let source = game.create_object_from_definition(&def.build(), alice, Zone::Battlefield);
        let target = counter_creature(&mut game, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(source, alice);
        RegenerateEffect::new(ChooseSpec::SpecificObject(target), Until::EndOfTurn)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(!game.is_tapped(target));
        DealDamageEffect::new(
            if mode == 0 { 4 } else { 1 },
            ChooseSpec::SpecificObject(target),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        if mode == 2 {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(target),
                Modification::ModifyToughness(-4),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut ctx)
            .unwrap();
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.battlefield.contains(&target),
            mode != 2,
            "CR 704.5f-h: mode={mode} (lethal damage, deathtouch, zero toughness)"
        );
        if mode != 2 {
            assert!(game.is_tapped(target));
            assert_eq!(game.damage_on(target), 0);
            // A regeneration shield is consumed by the first destruction.
            DealDamageEffect::new(4, ChooseSpec::SpecificObject(target))
                .execute(&mut game, &mut ctx)
                .unwrap();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                !game.battlefield.contains(&target),
                "CR 701.19a: one shield cannot replace two destructions"
            );
        }
    }
}

#[test]
fn cr_303_4c_d_illegal_auras_go_to_owners_graveyard() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ObjectFilter, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for case in 0..6 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = counter_creature(&mut game, Zone::Battlefield);
        let def =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Audit Aura")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .enchants(ObjectFilter::creature())
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
        let aura = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let stable = game.object(aura).unwrap().stable_id;
        if case != 0 {
            assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        }
        match case {
            1 | 2 | 4 => {
                let (target, modification) = match case {
                    1 => (
                        host,
                        Modification::RemoveCardTypes(vec![CardType::Creature]),
                    ),
                    2 => (aura, Modification::AddCardTypes(vec![CardType::Creature])),
                    _ => (
                        aura,
                        Modification::ChangeController(PlayerId::from_index(1)),
                    ),
                };
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(target),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(aura, alice))
                .unwrap();
            }
            3 => {
                game.move_object_by_effect(host, Zone::Exile).unwrap();
            }
            _ => {}
        }
        if case == 5 {
            for (target, modification) in [
                (
                    aura,
                    Modification::ChangeController(PlayerId::from_index(1)),
                ),
                (
                    host,
                    Modification::RemoveCardTypes(vec![CardType::Creature]),
                ),
            ] {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(target),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(aura, alice))
                .unwrap();
            }
            assert_eq!(game.current_controller(aura), Some(PlayerId::from_index(1)));
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let dies = case != 4;
        assert_eq!(
            game.battlefield.contains(&aura),
            !dies,
            "CR 303.4c/d: case={case}"
        );
        if dies {
            assert!(
                game.player(alice).unwrap().graveyard.iter().any(|id| game
                    .object(*id)
                    .unwrap()
                    .stable_id
                    == stable)
            );
        } else {
            assert_eq!(
                game.object(aura).unwrap().attached_to,
                Some(AttachmentTarget::Object(host))
            );
        }
    }
}

#[test]
fn cr_303_4f_g_aura_entry_requires_a_legal_enchantment_host() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for origin in [Zone::Hand, Zone::Graveyard, Zone::Exile, Zone::Stack] {
        for legal_host in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host = if legal_host {
                Some(counter_creature(&mut game, Zone::Battlefield))
            } else {
                None
            };
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Entering Aura",
            )
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .enchants(ObjectFilter::creature())
            .build();
            let aura = game.create_object_from_definition(&def, alice, origin);
            let stable = game.object(aura).unwrap().stable_id;
            if origin == Zone::Stack {
                game.stack
                    .push(ironsmith::game_state::StackEntry::new(aura, alice));
            }
            // This is an effect moving a card, not resolution of the Aura spell.
            let source_def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Zone move source",
            )
            .card_types(vec![CardType::Artifact])
            .build();
            let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);

            MoveToZoneEffect::new(ChooseSpec::SpecificObject(aura), Zone::Battlefield, false)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            if let Some(host) = host {
                let entered = *game
                    .battlefield
                    .iter()
                    .find(|id| game.object(**id).unwrap().stable_id == stable)
                    .expect("Aura should enter with a legal host");
                assert_eq!(
                    game.object(entered).unwrap().attached_to,
                    Some(AttachmentTarget::Object(host))
                );
            } else if origin == Zone::Stack {
                assert!(
                    !game
                        .battlefield
                        .iter()
                        .any(|id| game.object(*id).unwrap().stable_id == stable),
                    "CR 303.4g: the Aura must not enter unattached before going to the graveyard"
                );
                assert!(
                    game.player(alice).unwrap().graveyard.iter().any(|id| game
                        .object(*id)
                        .unwrap()
                        .stable_id
                        == stable),
                    "CR 303.4g: a stack Aura with no legal host goes to its owner's graveyard"
                );
            } else {
                assert_eq!(
                    game.object(aura).map(|object| object.zone),
                    Some(origin),
                    "CR 303.4g: no legal host leaves Aura in {origin:?}, without changing object identity"
                );
            }
        }
    }
}

#[test]
fn cr_303_4f_stack_aura_moved_by_effect_attaches_to_available_host() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let host = counter_creature(&mut game, Zone::Battlefield);
    let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Stack Aura with host",
    )
    .card_types(vec![CardType::Enchantment])
    .subtypes(vec![Subtype::Aura])
    .enchants(ObjectFilter::creature())
    .build();
    let aura = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.stack
        .push(ironsmith::game_state::StackEntry::new(aura, alice));
    let stable = game.object(aura).unwrap().stable_id;
    // Execute another object's effect rather than resolving the Aura spell.
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(aura), Zone::Battlefield, false)
        .execute(&mut game, &mut EffectContext::new_default(host, alice))
        .unwrap();
    let entered = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().stable_id == stable)
        .unwrap();
    assert_eq!(
        game.object(entered).unwrap().attached_to,
        Some(AttachmentTarget::Object(host)),
        "CR 303.4f: stack origin does not exempt an Aura moved by a separate effect from choosing its host"
    );
}

#[test]
fn cr_303_4j_illegal_aura_reattachment_preserves_existing_host() {
    use ironsmith::effects::{AttachObjectsEffect, AttachToEffect};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for plural_path in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let old = counter_creature(&mut game, Zone::Battlefield);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Reattachment Aura",
        )
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .enchants(ObjectFilter::creature())
        .build();
        let aura = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(old)));
        let grave_creature = counter_creature(&mut game, Zone::Graveyard);
        for target in [aura, grave_creature] {
            let mut ctx = EffectContext::new_default(aura, alice);
            if plural_path {
                let _ = AttachObjectsEffect::new(
                    ChooseSpec::SpecificObject(aura),
                    ChooseSpec::SpecificObject(target),
                )
                .execute(&mut game, &mut ctx);
            } else {
                let _ = AttachToEffect::new(ChooseSpec::SpecificObject(target))
                    .execute(&mut game, &mut ctx);
            }
            assert_eq!(
                game.object(aura).unwrap().attached_to,
                Some(AttachmentTarget::Object(old)),
                "CR 303.4j: illegal reattachment must not move an existing Aura"
            );
            assert!(game.object(old).unwrap().attachments.contains(&aura));
        }
    }
}

#[test]
fn cr_305_1_2_3_land_play_legality_respects_turn_phase_stack_and_allowance() {
    use ironsmith::{CardId, CardType, Effect, LegalAction, Phase, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for active in [alice, bob] {
        for phase in [
            Phase::Beginning,
            Phase::FirstMain,
            Phase::Combat,
            Phase::NextMain,
            Phase::Ending,
        ] {
            for occupied in [false, true] {
                for played in [0, 1, 2] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = active;
                    game.turn.priority_player = Some(alice);
                    game.turn.phase = phase;
                    game.turn.step = None;
                    game.player_mut(alice).unwrap().lands_played_this_turn = played;
                    let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                        CardId::new(),
                        "Timing audit land",
                    )
                    .card_types(vec![CardType::Land])
                    .build();
                    let land = game.create_object_from_definition(&def, alice, Zone::Hand);
                    if occupied {
                        let source = counter_creature(&mut game, Zone::Battlefield);
                        game.stack.push(ironsmith::game_state::StackEntry::ability(
                            source,
                            alice,
                            vec![Effect::gain_life(1)],
                        ));
                    }
                    let legal = ironsmith::compute_legal_actions(&game, alice)
                        .unwrap()
                        .iter()
                        .any(|a| matches!(a,LegalAction::PlayLand{land_id} if *land_id==land));
                    let expected = active == alice
                        && matches!(phase, Phase::FirstMain | Phase::NextMain)
                        && !occupied
                        && played == 0;
                    assert_eq!(
                        legal, expected,
                        "CR 305.1-3: active={active:?}, phase={phase:?}, occupied={occupied}, played={played}"
                    );
                }
            }
        }
    }
}

#[test]
fn cr_305_2a_land_allowance_recalculates_without_resetting_plays() {
    use ironsmith::{Ability, CardId, CardType, LegalAction, Phase, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.player_mut(alice).unwrap().lands_played_this_turn = 2;
    let land = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Allowance audit land",
    )
    .card_types(vec![CardType::Land])
    .build();
    let land = game.create_object_from_definition(&land, alice, Zone::Hand);
    let grant =
        ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Extra land grant")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::additional_land_plays(1),
            ))
            .build();
    let first = game.create_object_from_definition(&grant, alice, Zone::Battlefield);
    let second = game.create_object_from_definition(&grant, alice, Zone::Battlefield);
    for (remove, expected) in [(None, true), (Some(first), false), (Some(second), false)] {
        if let Some(id) = remove {
            game.move_object_by_effect(id, Zone::Graveyard).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 2);
        assert_eq!(
            ironsmith::compute_legal_actions(&game, alice)
                .unwrap()
                .iter()
                .any(|a| matches!(a,LegalAction::PlayLand{land_id} if *land_id==land)),
            expected,
            "CR 305.2a/b: compare current total allowance with existing play count"
        );
    }
    game.create_object_from_definition(&grant, alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert!(
        !ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .iter()
            .any(|a| matches!(a,LegalAction::PlayLand{land_id} if *land_id==land)),
        "A new grant brings allowance back to two, not a fresh extra play"
    );
}

#[test]
fn cr_305_6_all_basic_land_type_combinations_grant_correct_mana() {
    use ironsmith::{AbilityKind, CardId, CardType, ManaSymbol, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let kinds = [
        (Subtype::Plains, ManaSymbol::White),
        (Subtype::Island, ManaSymbol::Blue),
        (Subtype::Swamp, ManaSymbol::Black),
        (Subtype::Mountain, ManaSymbol::Red),
        (Subtype::Forest, ManaSymbol::Green),
    ];
    for mask in 0..32 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let subtypes = kinds
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, p)| p.0)
            .collect();
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Intrinsic mana land",
        )
        .card_types(vec![CardType::Land])
        .subtypes(subtypes)
        .build();
        let land = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let abilities = game.current_abilities(land).unwrap();
        let outputs: Vec<_> = abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(a) if a.is_mana_ability() => Some(a.mana_symbols().to_vec()),
                _ => None,
            })
            .collect();
        let expected: Vec<_> = kinds
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, p)| vec![p.1])
            .collect();
        assert_eq!(outputs, expected, "CR 305.6: subtype mask={mask}");
        for (index, ability) in abilities.iter().enumerate() {
            if let AbilityKind::Activated(a) = &ability.kind {
                if !a.is_mana_ability() {
                    continue;
                }
                game.untap(land);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::special_actions::perform_activate_mana_ability(
                    &mut game, alice, land, index, &mut dm,
                )
                .unwrap();
                assert!(game.is_tapped(land));
            }
        }
        let pool = &game.player(alice).unwrap().mana_pool;
        assert_eq!(
            [pool.white, pool.blue, pool.black, pool.red, pool.green],
            [0, 1, 2, 3, 4].map(|i| if mask & (1 << i) != 0 { 1 } else { 0 }),
            "CR 305.6: actual mana production, mask={mask}"
        );
    }
}

#[test]
fn cr_305_7_setting_basic_land_type_removes_printed_but_preserves_granted_abilities() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, StaticAbility, Subtype, Supertype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for replace in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Land ability audit",
        )
        .card_types(vec![CardType::Land, CardType::Creature])
        .subtypes(vec![Subtype::Forest])
        .supertypes(vec![Supertype::Legendary, Supertype::Snow])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .build();
        let land = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let mut ctx = EffectContext::new_default(land, alice);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(land),
            Modification::AddAbility(StaticAbility::flying()),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        ironsmith::effects::BecomeBasicLandTypeChoiceEffect {
            target: ironsmith::ChooseSpec::SpecificObject(land),
            duration: Until::EndOfTurn,
            chooser: PlayerFilter::You,
            fixed_subtype: Some(Subtype::Island),
            preserve_other_types: !replace,
            allowed_subtypes: vec![],
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert!(
            game.current_subtypes(land)
                .unwrap()
                .contains(&Subtype::Island)
        );
        assert_eq!(
            game.current_subtypes(land)
                .unwrap()
                .contains(&Subtype::Forest),
            !replace
        );
        assert!(
            game.current_card_types(land)
                .unwrap()
                .contains(&CardType::Creature)
        );
        assert!(
            game.current_supertypes(land)
                .unwrap()
                .contains(&Supertype::Snow)
        );
        assert!(
            game.current_supertypes(land)
                .unwrap()
                .contains(&Supertype::Legendary)
        );
        assert!(
            game.object_has_ability(land, &StaticAbility::flying()),
            "Externally granted ability survives"
        );
        assert_eq!(
            game.object_has_ability(land, &StaticAbility::vigilance()),
            !replace,
            "CR 305.7: setting a basic land type removes printed abilities; adding a type retains them"
        );
    }
}

#[test]
fn cr_306_5b_intrinsic_entry_counters_obey_ability_loss() {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, CounterType, ObjectFilter, StaticAbility, Zone,
    };
    let alice = PlayerId::from_index(0);
    for (kind, counter) in [
        (CardType::Planeswalker, CounterType::Loyalty),
        (CardType::Battle, CounterType::Defense),
    ] {
        for creature in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Ability suppression source",
            )
            .card_types(vec![CardType::Enchantment])
            .build();
            let source = game.create_object_from_definition(&source, alice, Zone::Battlefield);
            game.effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::new(
                    source,
                    alice,
                    EffectTarget::Filter(ObjectFilter::creature()),
                    Modification::RemoveAllAbilities,
                ));
            let mut types = vec![kind];
            if creature {
                types.push(CardType::Creature);
            }
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Intrinsic counter entrant",
            )
            .card_types(types)
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(
                StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 2),
            ));
            let def = if kind == CardType::Planeswalker {
                def.loyalty(5).build()
            } else {
                def.defense(5).build()
            };
            let card = game.create_object_from_definition(&def, alice, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            let entrant = *game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().stable_id == stable)
                .unwrap();
            assert_eq!(
                game.counter_count(entrant, counter),
                if creature { 0 } else { 5 },
                "CR 306.5b / 310.4b: intrinsic counters are from an ability, kind={kind:?}, creature={creature}"
            );
            assert_eq!(
                game.counter_count(entrant, CounterType::PlusOnePlusOne),
                if creature { 0 } else { 2 },
                "Control: printed entry ability is suppressed in the same domain"
            );
            game.remove_counters(entrant, counter, 5, None, None);
            let departed = game.move_object_by_effect(entrant, Zone::Exile).unwrap();
            assert_eq!(game.counter_count(departed, counter), 0);
            MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(departed),
                Zone::Battlefield,
                false,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            let returned = *game
                .battlefield
                .iter()
                .find(|id| game.object(**id).unwrap().stable_id == stable)
                .unwrap();
            assert_ne!(returned, entrant);
            assert_eq!(
                game.counter_count(returned, counter),
                if creature { 0 } else { 5 },
                "A new entry applies the intrinsic replacement anew, subject to current ability loss"
            );
        }
    }
}

#[test]
fn cr_606_3_loyalty_restrictions_apply_to_every_permanent_type() {
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, CounterType, Effect, LegalAction, Phase, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for kind in [CardType::Planeswalker, CardType::Artifact] {
        for active in [alice, bob] {
            for phase in [Phase::FirstMain, Phase::Combat] {
                for occupied in [false, true] {
                    for used in [false, true] {
                        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                        game.turn.active_player = active;
                        game.turn.priority_player = Some(alice);
                        game.turn.phase = phase;
                        game.turn.step = None;
                        // AnyTime template ensures the loyalty rule itself supplies the timing restriction.
                        let mut ability = Ability::activated(
                            TotalCost::from_cost(Cost::add_counters(CounterType::Loyalty, 1)),
                            vec![Effect::gain_life(1)],
                        );
                        if let AbilityKind::Activated(a) = &mut ability.kind {
                            a.is_loyalty_ability = true;
                        }
                        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                            CardId::new(),
                            "Loyalty timing permanent",
                        )
                        .card_types(vec![kind])
                        .loyalty(3)
                        .with_ability(ability.clone())
                        .with_ability(ability)
                        .build();
                        let id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
                        if used {
                            game.record_loyalty_ability_activation(id);
                        }
                        if occupied {
                            game.stack.push(ironsmith::game_state::StackEntry::ability(
                                id,
                                alice,
                                vec![Effect::gain_life(1)],
                            ));
                        }
                        let actions = ironsmith::compute_legal_actions(&game, alice).unwrap();
                        let count=actions.iter().filter(|action|matches!(action,LegalAction::ActivateAbility{source,..} if *source==id)).count();
                        let allowed =
                            active == alice && phase == Phase::FirstMain && !occupied && !used;
                        assert_eq!(
                            count,
                            if allowed { 2 } else { 0 },
                            "CR 606.3: kind={kind:?}, active={active:?}, phase={phase:?}, occupied={occupied}, used={used}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cr_606_3_loyalty_usage_survives_control_changes_but_not_reentry() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::{ApplyContinuousEffect, MoveToZoneEffect};
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, CounterType, Effect, LegalAction,
        Phase, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let mut ability = Ability::activated(
        TotalCost::from_cost(Cost::add_counters(CounterType::Loyalty, 1)),
        vec![Effect::gain_life(1)],
    );
    if let AbilityKind::Activated(a) = &mut ability.kind {
        a.is_loyalty_ability = true;
    }
    let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::new(),
        "Loyalty identity subject",
    )
    .card_types(vec![CardType::Planeswalker])
    .loyalty(3)
    .with_ability(ability)
    .build();
    let id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.record_loyalty_ability_activation(id);
    for controller in [bob, alice] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(id),
            Modification::ChangeController(controller),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(id, alice))
        .unwrap();
        assert_eq!(game.loyalty_activations_this_turn(id), 1);
    }
    assert!(
        !ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .iter()
            .any(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==id))
    );
    let stable = game.object(id).unwrap().stable_id;
    let exile = game.move_object_by_effect(id, Zone::Exile).unwrap();
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(exile), Zone::Battlefield, false)
        .execute(&mut game, &mut EffectContext::new_default(exile, alice))
        .unwrap();
    let returned = *game
        .battlefield
        .iter()
        .find(|id| game.object(**id).unwrap().stable_id == stable)
        .unwrap();
    assert_ne!(returned, id);
    assert_eq!(game.loyalty_activations_this_turn(returned), 0);
    assert!(
        ironsmith::compute_legal_actions(&game, alice)
            .unwrap()
            .iter()
            .any(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==returned)),
        "CR 606.3 / 400.7: the returned permanent has not activated a loyalty ability"
    );
}

#[test]
fn cr_606_6_negative_loyalty_cost_requires_enough_counters() {
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, CounterType, Effect, LegalAction, Phase, Zone,
    };
    let alice = PlayerId::from_index(0);
    for counters in 0..=4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let mut ability = Ability::activated(
            TotalCost::from_cost(Cost::remove_counters(CounterType::Loyalty, 3)),
            vec![Effect::gain_life(1)],
        );
        if let AbilityKind::Activated(a) = &mut ability.kind {
            a.is_loyalty_ability = true;
        }
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Loyalty cost subject",
        )
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .with_ability(ability)
        .build();
        let id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        game.remove_counters(id, CounterType::Loyalty, 4 - counters, None, None);
        assert_eq!(game.counter_count(id, CounterType::Loyalty), counters);
        assert_eq!(
            ironsmith::compute_legal_actions(&game, alice)
                .unwrap()
                .iter()
                .any(|a| matches!(a,LegalAction::ActivateAbility{source,..} if *source==id)),
            counters >= 3,
            "CR 606.6: -3 cost with {counters} counters"
        );
    }
}

#[test]
fn cr_111_7_8_departed_tokens_cannot_move_again_and_cease_to_exist() {
    use ironsmith::effects::{CreateTokenCopyEffect, MoveToZoneEffect};
    use ironsmith::{ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for first_zone in [
        Zone::Graveyard,
        Zone::Exile,
        Zone::Hand,
        Zone::Library,
        Zone::Command,
    ] {
        for second_zone in [
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Hand,
            Zone::Library,
        ] {
            if first_zone == second_zone {
                continue;
            }
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = counter_creature(&mut game, Zone::Battlefield);
            CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source))
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            let token = *game.battlefield.iter().find(|id| **id != source).unwrap();
            assert!(game.object(token).unwrap().card.is_none());
            assert_eq!(
                game.object(token).unwrap().kind,
                ironsmith::object::ObjectKind::Token
            );
            // No SBA between these instructions: the departed token still exists.
            let departed = game.move_object_by_effect(token, first_zone).unwrap();
            assert_eq!(game.object(departed).unwrap().zone, first_zone);
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(departed), second_zone, false)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            assert_eq!(
                game.object(departed).map(|o| o.zone),
                Some(first_zone),
                "CR 111.8: token must stay in {first_zone:?}, attempted {second_zone:?}"
            );
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                game.object(departed).is_none(),
                "CR 111.7 / 704.5d: departed token ceases to exist; first={first_zone:?}, attempted={second_zone:?}"
            );
            assert!(
                game.object(source).is_some(),
                "The original card is unaffected"
            );
        }
    }
}

#[test]
fn cr_111_2_token_owner_is_creator_and_survives_control_change() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, CreateTokenCopyEffect, MoveToZoneEffect};
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let original = counter_creature(&mut game, Zone::Battlefield);
    CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(original))
        .execute(&mut game, &mut EffectContext::new_default(original, bob))
        .unwrap();
    let token = *game.battlefield.iter().find(|id| **id != original).unwrap();
    assert_eq!(game.object(token).unwrap().owner, bob);
    assert_eq!(game.current_controller(token), Some(bob));
    let stable = game.object(token).unwrap().stable_id;
    ApplyContinuousEffect::new(
        EffectTarget::Specific(token),
        Modification::ChangeController(alice),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(original, alice))
    .unwrap();
    assert_eq!(game.current_controller(token), Some(alice));
    assert_eq!(game.object(token).unwrap().owner, bob);
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(token), Zone::Graveyard, false)
        .execute(&mut game, &mut EffectContext::new_default(original, alice))
        .unwrap();
    assert!(
        game.player(bob)
            .unwrap()
            .graveyard
            .iter()
            .any(|id| game.object(*id).unwrap().stable_id == stable)
    );
    assert!(game.player(alice).unwrap().graveyard.is_empty());
}

#[test]
fn cr_111_5_token_copy_of_instant_or_sorcery_is_not_created() {
    use ironsmith::effects::CreateTokenCopyEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    for kind in [
        CardType::Artifact,
        CardType::Creature,
        CardType::Instant,
        CardType::Sorcery,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Token copy type subject",
        )
        .card_types(vec![kind])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
        let source = game.create_object_from_definition(&def, alice, Zone::Graveyard);
        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(source))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        results.push((kind, game.battlefield.len()));
    }
    assert_eq!(
        results,
        vec![
            (CardType::Artifact, 1),
            (CardType::Creature, 1),
            (CardType::Instant, 0),
            (CardType::Sorcery, 0)
        ],
        "CR 111.5: instant and sorcery token copies must not be created"
    );
}

#[test]
fn cr_111_4_token_name_and_subtypes_change_independently() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, CreateTokenEffect};
    use ironsmith::{CardId, CardType, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    for rename_first in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let blueprint =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Goblin Token")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Goblin])
                .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
                .build();
        CreateTokenEffect::one(blueprint)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let token = *game.battlefield.iter().find(|id| **id != source).unwrap();
        assert_eq!(
            game.object(token).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        );
        let mut mods = vec![
            Modification::SetSubtypes(vec![Subtype::Elf]),
            Modification::SetName("Named audit token".into()),
        ];
        if rename_first {
            mods.reverse();
        }
        for (index, modification) in mods.into_iter().enumerate() {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(token),
                modification,
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            let renamed = rename_first || index == 1;
            let retyped = !rename_first || index == 1;
            assert_eq!(
                game.current_name(token).as_deref(),
                Some(if renamed {
                    "Named audit token"
                } else {
                    "Goblin Token"
                })
            );
            assert_eq!(
                game.current_subtypes(token).unwrap(),
                vec![if retyped {
                    Subtype::Elf
                } else {
                    Subtype::Goblin
                }],
                "CR 111.4: changing name and subtype are independent"
            );
        }
        game.add_counters(token, ironsmith::CounterType::PlusOnePlusOne, 2);
        ironsmith::effects::CreateTokenCopyEffect::one(ironsmith::ChooseSpec::SpecificObject(
            token,
        ))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        let copy = *game
            .battlefield
            .iter()
            .find(|id| **id != source && **id != token)
            .unwrap();
        assert_eq!(game.current_name(copy).as_deref(), Some("Goblin Token"));
        assert_eq!(game.current_subtypes(copy).unwrap(), vec![Subtype::Goblin]);
        assert_eq!(
            (game.current_power(copy), game.current_toughness(copy)),
            (Some(1), Some(1)),
            "CR 111.3: creation characteristics are copiable; later counters are not"
        );
    }
}

#[test]
fn cr_400_3_control_changes_do_not_redirect_owner_zones() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, MoveToZoneEffect};
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for destination in [Zone::Hand, Zone::Library, Zone::Graveyard] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let object = counter_creature(&mut game, Zone::Battlefield);
        let stable = game.object(object).unwrap().stable_id;
        ApplyContinuousEffect::new(
            EffectTarget::Specific(object),
            Modification::ChangeController(bob),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(object, bob))
        .unwrap();
        assert_eq!(game.current_controller(object), Some(bob));
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(object), destination, false)
            .execute(&mut game, &mut EffectContext::new_default(object, bob))
            .unwrap();
        for owner in [alice, bob] {
            let player = game.player(owner).unwrap();
            let ids = match destination {
                Zone::Hand => &player.hand,
                Zone::Library => &player.library,
                _ => &player.graveyard,
            };
            assert_eq!(
                ids.iter()
                    .any(|id| game.object(*id).unwrap().stable_id == stable),
                owner == alice,
                "CR 400.3: destination={destination:?}, zone owner={owner:?}"
            );
        }
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn cr_400_4a_instant_and_sorcery_cards_cannot_enter_battlefield() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for kind in [CardType::Instant, CardType::Sorcery] {
        for origin in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Command,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Nonpermanent entry subject",
            )
            .card_types(vec![kind])
            .build();
            let object = game.create_object_from_definition(&def, alice, origin);
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(object), Zone::Battlefield, false)
                .execute(&mut game, &mut EffectContext::new_default(object, alice))
                .unwrap();
            assert_eq!(
                game.object(object).map(|o| o.zone),
                Some(origin),
                "CR 400.4a: {kind:?} remains in {origin:?} with unchanged object identity"
            );
            assert!(game.battlefield.is_empty());
        }
    }
}

#[test]
fn cr_400_4b_conspiracy_cannot_leave_command_zone() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, Zone};
    let alice = PlayerId::from_index(0);
    for kind in [CardType::Conspiracy] {
        for destination in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Stack,
            Zone::Battlefield,
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Command zone type subject",
            )
            .card_types(vec![kind])
            .build();
            game.enable_conspiracy(vec![(
                alice,
                vec![ironsmith::game_state::ConspiracySetupCard {
                    definition: def,
                    agenda_names: vec![],
                }],
            )])
            .unwrap();
            let object = game.conspiracy_cards()[0];
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(object), destination, false)
                .execute(&mut game, &mut EffectContext::new_default(object, alice))
                .unwrap();
            assert_eq!(
                game.object(object).map(|o| o.zone),
                Some(Zone::Command),
                "CR 400.4b: {kind:?} must not move to {destination:?}"
            );
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn cr_400_8_10_reexile_and_command_reentry_create_new_objects() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Exile, Zone::Command] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = counter_creature(&mut game, zone);
        let stable = game.object(card).unwrap().stable_id;
        game.add_counters(card, CounterType::Charge, 2);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), zone, false)
            .execute(&mut game, &mut EffectContext::new_default(card, alice))
            .unwrap();
        assert!(
            game.object(card).is_none(),
            "CR 400.8/10: same-zone instruction in {zone:?} creates a new object"
        );
        let ids = if zone == Zone::Exile {
            &game.exile
        } else {
            &game.command_zone
        };
        let new = ids
            .iter()
            .filter_map(|id| game.object(*id))
            .find(|o| o.stable_id == stable)
            .unwrap();
        assert_ne!(new.id, card);
        assert_eq!(new.zone, zone);
        assert_eq!(game.counter_count(new.id, CounterType::Charge), 0);
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn cr_401_4_library_owner_orders_simultaneously_placed_cards() {
    use ironsmith::decisions::context::OrderContext;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, ObjectId, Zone};
    struct OrderByName {
        seen: Vec<PlayerId>,
    }
    impl DecisionMaker for OrderByName {
        fn decide_order(&mut self, game: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
            assert_eq!(ctx.items.len(), 2);
            assert!(
                ctx.items
                    .iter()
                    .all(|(id, _)| game.object(*id).unwrap().owner == ctx.player),
                "CR 401.4: each library owner orders only their own cards"
            );
            self.seen.push(ctx.player);
            let mut items = ctx.items.clone();
            items.sort_by(|a, b| b.1.cmp(&a.1));
            items.into_iter().map(|p| p.0).collect()
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for top in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for owner in [alice, bob] {
            for name in ["A library subject", "Z library subject"] {
                let def =
                    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                        .build();
                game.create_object_from_definition(&def, owner, Zone::Battlefield);
            }
        }
        let source = game.battlefield[0];
        let mut dm = OrderByName { seen: vec![] };
        MoveToZoneEffect::new(
            ChooseSpec::All(ObjectFilter::creature()),
            Zone::Library,
            top,
        )
        .with_library_order(ironsmith::effects::LibraryPlacementOrder::Owners)
        .execute(
            &mut game,
            &mut EffectContext::new_default(source, alice).with_decision_maker(&mut dm),
        )
        .unwrap();
        dm.seen.sort_by_key(|p| p.0);
        assert_eq!(dm.seen, vec![alice, bob]);
        for owner in [alice, bob] {
            let mut names: Vec<_> = game
                .player(owner)
                .unwrap()
                .library
                .iter()
                .map(|id| game.object(*id).unwrap().name.to_string())
                .collect();
            if top {
                names.reverse();
            }
            assert_eq!(
                names,
                vec!["Z library subject", "A library subject"],
                "CR 401.4: selected first card belongs at the requested edge; top={top}"
            );
        }
    }
}

#[test]
fn cr_400_7_library_repositioning_preserves_object_identity() {
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    for top in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        counter_creature(&mut game, Zone::Library);
        let middle = counter_creature(&mut game, Zone::Library);
        counter_creature(&mut game, Zone::Library);
        game.add_counters(middle, CounterType::Charge, 2);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(middle), Zone::Library, top)
            .execute(&mut game, &mut EffectContext::new_default(middle, alice))
            .unwrap();
        assert_eq!(game.object(middle).unwrap().zone, Zone::Library);
        assert_eq!(game.counter_count(middle, CounterType::Charge), 2);
        let library = &game.player(alice).unwrap().library;
        assert_eq!(library.len(), 3);
        assert_eq!(
            if top {
                library.last().copied()
            } else {
                library.first().copied()
            },
            Some(middle)
        );
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn cr_402_2_cleanup_uses_timestamp_ordered_hand_size_effects() {
    use ironsmith::{Ability, CardId, CardType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let inc = || StaticAbility::increase_maximum_hand_size(PlayerFilter::You, 2);
    let dec = || StaticAbility::reduce_maximum_hand_size(PlayerFilter::You, 3);
    let set = || StaticAbility::set_maximum_hand_size(PlayerFilter::You, 5);
    let unlimited = || StaticAbility::no_maximum_hand_size();
    for (abilities, remaining) in [
        (vec![], 7),
        (vec![inc()], 9),
        (vec![dec()], 4),
        (vec![inc(), set(), dec()], 2),
        (vec![dec(), set(), inc()], 7),
        (vec![unlimited(), dec()], 10),
        (vec![unlimited(), set()], 5),
        (vec![set(), unlimited()], 10),
        (
            vec![StaticAbility::reduce_maximum_hand_size(
                PlayerFilter::You,
                9,
            )],
            0,
        ),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        for ability in abilities {
            let def = ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Hand size rule",
            )
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(ability))
            .build();
            game.create_object_from_definition(&def, alice, Zone::Battlefield);
        }
        for _ in 0..10 {
            counter_creature(&mut game, Zone::Hand);
        }
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            10,
            "Hand limit does not immediately discard cards"
        );
        let choice = ironsmith::turn::get_cleanup_discard_spec(&game);
        if remaining == 10 {
            assert!(choice.is_none());
            continue;
        }
        let (chooser, spec) = choice.expect("Cleanup requires discarding excess cards");
        assert_eq!(chooser, alice);
        assert_eq!(
            spec.count,
            10 - remaining,
            "CR 402.2 / 613.11: discard count follows timestamp-ordered rules"
        );
        let selected = spec.hand[..spec.count].to_vec();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::turn::apply_cleanup_discard(&mut game, &selected, &mut dm).unwrap();
        assert_eq!(game.player(alice).unwrap().hand.len(), remaining);
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 10 - remaining);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_514_2_cleanup_clears_damage_and_expires_effects_on_phased_out_permanents() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect};
    use ironsmith::{ChooseSpec, CounterType, Until, Zone};
    let alice = PlayerId::from_index(0);
    for phased in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let object = counter_creature(&mut game, Zone::Battlefield);
        game.add_counters(object, CounterType::PlusOnePlusOne, 1);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(object),
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(object, alice))
        .unwrap();
        DealDamageEffect::new(3, ChooseSpec::SpecificObject(object))
            .execute(&mut game, &mut EffectContext::new_default(object, alice))
            .unwrap();
        assert_eq!(game.current_toughness(object), Some(7));
        if phased {
            game.phase_out(object);
        }
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.damage_on(object), 0, "CR 514.2: phased={phased}");
        assert_eq!(game.is_phased_out(object), phased);
        if phased {
            game.phase_in(object);
        }
        assert_eq!(
            (game.current_power(object), game.current_toughness(object)),
            (Some(4), Some(5))
        );
        assert_eq!(game.counter_count(object, CounterType::PlusOnePlusOne), 1);
    }
}

#[test]
fn cr_514_1_temporary_hand_size_applies_before_cleanup_effect_expiration() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{StaticAbility, Until, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    let source = counter_creature(&mut game, Zone::Battlefield);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::AddAbility(StaticAbility::increase_maximum_hand_size(
            PlayerFilter::You,
            2,
        )),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    for _ in 0..10 {
        counter_creature(&mut game, Zone::Hand);
    }
    game.refresh_continuous_state().unwrap();
    let (_, spec) = ironsmith::turn::get_cleanup_discard_spec(&game).unwrap();
    assert_eq!(
        spec.count, 1,
        "CR 514.1: temporary maximum nine still applies during discard"
    );
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::turn::apply_cleanup_discard(&mut game, &spec.hand[..1], &mut dm).unwrap();
    ironsmith::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.player(alice).unwrap().max_hand_size, 7);
    assert_eq!(
        game.player(alice).unwrap().hand.len(),
        9,
        "CR 514.2: effect expiration does not itself perform another discard"
    );
}

#[test]
fn cr_502_1_and_502_3_phasing_precedes_untap_and_stun_replacement() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, CounterType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for controller in [alice, bob] {
        for initially_phased_out in [false, true] {
            for has_phasing in [false, true] {
                for blocked in [false, true] {
                    for stun in [0, 1, 2] {
                        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                        game.turn.active_player = alice;
                        let mut builder =
                            CardDefinitionBuilder::new(CardId::new(), "Untap subject")
                                .card_types(vec![CardType::Artifact]);
                        if has_phasing {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::phasing()));
                        }
                        if blocked {
                            builder = builder.with_ability(Ability::static_ability(
                                StaticAbility::doesnt_untap(),
                            ));
                        }
                        let object = game.create_object_from_definition(
                            &builder.build(),
                            controller,
                            Zone::Battlefield,
                        );
                        game.tap(object);
                        if stun > 0 {
                            game.add_counters(object, CounterType::Stun, stun);
                        }
                        if initially_phased_out {
                            game.phase_out(object);
                        }
                        ironsmith::turn::execute_untap_step(&mut game);
                        let phased_out = if controller == alice {
                            !initially_phased_out && has_phasing
                        } else {
                            initially_phased_out
                        };
                        let attempts_untap = controller == alice && !phased_out && !blocked;
                        assert_eq!(
                            (
                                game.is_phased_out(object),
                                game.is_tapped(object),
                                game.counter_count(object, CounterType::Stun)
                            ),
                            (
                                phased_out,
                                !attempts_untap || stun > 0,
                                stun - u32::from(attempts_untap && stun > 0)
                            ),
                            "CR 502.1/502.3/122.1d: controller={controller:?}, initially phased={initially_phased_out}, phasing={has_phasing}, blocked={blocked}, stun={stun}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cr_502_2_skipping_untap_step_skips_day_night_check() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for skip in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.set_daytime(true);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Untap skip source")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(
                StaticAbility::players_skip_untap_steps(PlayerFilter::Any),
            ))
            .build();
        if skip {
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        }
        let subject = CardDefinitionBuilder::new(CardId::new(), "Tapped subject")
            .card_types(vec![CardType::Artifact])
            .build();
        let permanent = game.create_object_from_definition(&subject, bob, Zone::Battlefield);
        game.tap(permanent);
        game.refresh_continuous_state().unwrap();
        // No player cast spells in the completed turn. Advance through the same
        // state transition the turn runner uses, then execute the untap entry.
        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut runner =
            ironsmith::TurnRunner::from_state_for_sync(ironsmith::TurnRunnerState::BeginTurn);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            ironsmith::TurnAction::Continue
        ));
        assert_eq!(
            game.is_tapped(permanent),
            skip,
            "Confirm the step was actually skipped"
        );
        results.push(game.is_daytime());
    }
    assert_eq!(
        results,
        vec![false, true],
        "CR 502.2 / 731.2 / 614.10a: day/night check belongs to the untap step and cannot happen when that step is skipped"
    );
}

#[test]
fn cr_731_2_day_night_thresholds_use_each_previous_active_team_members_casts() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::events::{EventKind, SpellCastEvent};
    use ironsmith::triggers::TriggerEvent;
    use ironsmith::{CardId, CardType, ObjectSnapshot, Zone};
    let players: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    for shared in [false, true] {
        for initial_day in [None, Some(true), Some(false)] {
            for first_count in 0..4 {
                for second_count in 0..4 {
                    for opponent_count in [0, 2] {
                        let mut game =
                            GameState::new((0..4).map(|i| format!("Player {i}")).collect(), 20);
                        if shared {
                            game.enable_two_headed_giant(vec![
                                vec![players[0], players[1]],
                                vec![players[2], players[3]],
                            ])
                            .unwrap();
                        }
                        game.turn.active_player = players[0];
                        if let Some(day) = initial_day {
                            game.set_daytime(day);
                        }
                        for (player, count) in [
                            (players[0], first_count),
                            (players[1], second_count),
                            (players[2], opponent_count),
                        ] {
                            for _ in 0..count {
                                let definition =
                                    CardDefinitionBuilder::new(CardId::new(), "Completed cast")
                                        .card_types(vec![CardType::Instant])
                                        .build();
                                let spell = game.create_object_from_definition(
                                    &definition,
                                    player,
                                    Zone::Graveyard,
                                );
                                let snapshot =
                                    ObjectSnapshot::from_object(game.object(spell).unwrap(), &game);
                                let provenance = game
                                    .provenance_graph_mut()
                                    .alloc_root_event(EventKind::SpellCast);
                                let event = TriggerEvent::new_with_provenance(
                                    SpellCastEvent::new_with_snapshot(
                                        spell,
                                        player,
                                        Zone::Hand,
                                        snapshot.clone(),
                                    ),
                                    provenance,
                                );
                                game.turn_store.turn_history.record_event(
                                    &event,
                                    Some(snapshot),
                                    None,
                                );
                            }
                            assert_eq!(
                                game.turn_store.turn_history.spells_cast_by_player(player),
                                count
                            );
                        }
                        game.next_turn();
                        let mut queue = ironsmith::triggers::TriggerQueue::new();
                        let mut runner = ironsmith::TurnRunner::from_state_for_sync(
                            ironsmith::TurnRunnerState::BeginTurn,
                        );
                        assert!(matches!(
                            runner.advance(&mut game, &mut queue).unwrap(),
                            ironsmith::TurnAction::Continue
                        ));
                        let max_count = if shared {
                            first_count.max(second_count)
                        } else {
                            first_count
                        };
                        let expected_day =
                            initial_day.map(|day| if day { max_count > 0 } else { max_count >= 2 });
                        let actual_day = game.has_day_night().then(|| game.is_daytime());
                        assert_eq!(
                            actual_day, expected_day,
                            "CR 731.2a-c: shared={shared}, initial={initial_day:?}, casts={first_count}/{second_count}/{opponent_count}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cr_106_4_mana_empties_at_each_runner_boundary_without_life_loss() {
    use ironsmith::{ManaSymbol, Phase, Step, TurnRunner, TurnRunnerState as State};
    let cases = [
        (State::UntapEndMana, Phase::Beginning, Some(Step::Untap)),
        (State::UpkeepPriority, Phase::Beginning, Some(Step::Upkeep)),
        (State::DrawPriority, Phase::Beginning, Some(Step::Draw)),
        (State::FirstMainPriority, Phase::FirstMain, None),
        (
            State::BeginCombatPriority,
            Phase::Combat,
            Some(Step::BeginCombat),
        ),
        (
            State::DeclareAttackersPriority,
            Phase::Combat,
            Some(Step::DeclareAttackers),
        ),
        (
            State::DeclareBlockersPriority,
            Phase::Combat,
            Some(Step::DeclareBlockers),
        ),
        (
            State::CombatDamageFirstStrikePriority,
            Phase::Combat,
            Some(Step::CombatDamage),
        ),
        (
            State::CombatDamageRegularPriority,
            Phase::Combat,
            Some(Step::CombatDamage),
        ),
        (
            State::EndCombatPriority,
            Phase::Combat,
            Some(Step::EndCombat),
        ),
        (State::NextMainPriority, Phase::NextMain, None),
        (State::EndStepPriority, Phase::Ending, Some(Step::End)),
    ];
    let symbols = [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ];
    for (state, phase, step) in cases {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = phase;
        game.turn.step = step;
        let source = counter_creature(&mut game, ironsmith::Zone::Battlefield);
        for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
            for symbol in symbols {
                game.player_mut(player).unwrap().mana_pool.add(symbol, 2);
            }
            ironsmith::effects::RetainManaUntilEndOfTurnEffect::you()
                .with_color(Some(ironsmith::Color::Red))
                .execute(&mut game, &mut EffectContext::new_default(source, player))
                .unwrap();
        }
        let mut runner = TurnRunner::from_state_for_sync(state);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            ironsmith::TurnAction::Continue
        ));
        for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
            for symbol in symbols {
                assert_eq!(
                    game.player(player).unwrap().mana_pool.amount(symbol),
                    if symbol == ManaSymbol::Red { 2 } else { 0 },
                    "CR 106.4/500.5: {phase:?} {step:?} {player:?} {symbol:?}; only retained red mana survives"
                );
            }
            assert_eq!(
                game.player(player).unwrap().life,
                20,
                "Losing unspent mana does not itself lose life"
            );
        }
        assert!(game.stack.is_empty(), "Mana loss is a turn-based action");
    }
}

#[test]
fn cr_103_8_first_turn_skips_entire_draw_step_in_duel_and_two_headed_giant() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, Phase, Step, TurnAction, TurnRunner, TurnRunnerState,
        Zone,
    };
    let mut skipped_results = Vec::new();
    for (count, shared) in [(2, false), (3, false), (4, false), (4, true)] {
        for turn_number in [1, 2] {
            let players: Vec<_> = (0..count).map(PlayerId::from_index).collect();
            let mut game = GameState::new((0..count).map(|i| format!("Player {i}")).collect(), 20);
            if shared {
                game.enable_two_headed_giant(vec![players[..2].to_vec(), players[2..].to_vec()])
                    .unwrap();
            }
            game.turn.active_player = players[0];
            game.turn.turn_number = turn_number;
            game.turn.phase = Phase::Beginning;
            game.turn.step = Some(Step::Draw);
            let observer = CardDefinitionBuilder::new(CardId::new(), "Draw step observer")
                .card_types(vec![CardType::Enchantment])
                .with_ability(Ability::triggered(
                    Trigger::beginning_of_draw_step(PlayerFilter::You),
                    vec![Effect::gain_life(1)],
                ))
                .build();
            game.create_object_from_definition(&observer, players[0], Zone::Battlefield);
            for player in &players {
                let card = CardDefinitionBuilder::new(CardId::new(), "Top card")
                    .card_types(vec![CardType::Artifact])
                    .build();
                game.create_object_from_definition(&card, *player, Zone::Library);
            }
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Draw);
            let mut queue = TriggerQueue::new();
            let action = runner.advance(&mut game, &mut queue).unwrap();
            let skip = turn_number == 1 && (count == 2 || shared);
            for (index, player) in players.iter().enumerate() {
                let expected_draw = !skip && (index == 0 || (shared && index == 1));
                assert_eq!(
                    game.player(*player).unwrap().hand.len(),
                    usize::from(expected_draw),
                    "CR 504.1 / 103.8: players={count}, shared={shared}, turn={turn_number}, player={player:?}"
                );
            }
            let result = (
                matches!(action, TurnAction::RunPriority),
                queue.entries.len(),
            );
            if skip {
                skipped_results.push(result);
            } else {
                assert_eq!(
                    result,
                    (true, 1),
                    "An actual draw step grants priority and triggers the observer"
                );
            }
        }
    }
    assert_eq!(
        skipped_results,
        vec![(false, 0), (false, 0)],
        "CR 103.8a-b: skipping the first draw STEP suppresses priority and beginning-of-draw-step triggers, not just the card draw"
    );
}

#[test]
fn cr_503_1_and_513_1_step_entry_has_no_automatic_draw_untap_or_cleanup() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, ManaSymbol, Phase, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    for upkeep in [true, false] {
        let alice = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = if upkeep {
            Phase::Beginning
        } else {
            Phase::Ending
        };
        let trigger = if upkeep {
            Trigger::beginning_of_upkeep(PlayerFilter::You)
        } else {
            Trigger::beginning_of_end_step(PlayerFilter::You)
        };
        let observer = CardDefinitionBuilder::new(CardId::new(), "Step observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(trigger, vec![Effect::gain_life(1)]))
            .build();
        game.create_object_from_definition(&observer, alice, Zone::Battlefield);
        let creature = counter_creature(&mut game, Zone::Battlefield);
        game.tap(creature);
        ironsmith::effects::DealDamageEffect::new(
            1,
            ironsmith::ChooseSpec::SpecificObject(creature),
        )
        .execute(&mut game, &mut EffectContext::new_default(creature, alice))
        .unwrap();
        ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(creature),
            ironsmith::continuous::Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
            ironsmith::Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(creature, alice))
        .unwrap();
        counter_creature(&mut game, Zone::Library);
        for _ in 0..9 {
            counter_creature(&mut game, Zone::Hand);
        }
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 2);
        let state = if upkeep {
            TurnRunnerState::Upkeep
        } else {
            TurnRunnerState::EndStep
        };
        let mut runner = TurnRunner::from_state_for_sync(state);
        let mut queue = TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(game.turn.priority_player, Some(alice));
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(game.player(alice).unwrap().hand.len(), 9);
        assert_eq!(game.player(alice).unwrap().library.len(), 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 2);
        assert!(game.is_tapped(creature));
        assert_eq!(game.damage_on(creature), 1);
        assert_eq!(game.current_power(creature), Some(5));
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "Trigger has not resolved at step entry"
        );
    }
}

#[test]
fn cr_513_2_late_end_step_observer_waits_for_next_occurrence() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, TurnAction, TurnRunner, TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let observer = CardDefinitionBuilder::new(CardId::new(), "End step observer")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::gain_life(1)],
        ))
        .build();
    game.create_object_from_definition(&observer, alice, Zone::Battlefield);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
    let mut queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(queue.entries.len(), 1);
    // Simulate an entry during the already-begun end step, then drain the entry
    // events. A beginning-of-step trigger must not fire retroactively.
    game.create_object_from_definition(&observer, alice, Zone::Battlefield);
    ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
    assert_eq!(queue.entries.len(), 1);
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::Continue
    ));
    assert_eq!(queue.entries.len(), 1);
    game.next_turn();
    let mut next_runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
    let mut next_queue = TriggerQueue::new();
    assert!(matches!(
        next_runner.advance(&mut game, &mut next_queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(
        next_queue.entries.len(),
        2,
        "Both observers trigger at the next end-step occurrence"
    );
}

#[test]
fn cr_506_4_attackers_leave_combat_for_type_control_and_phasing_changes() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for case in 0..6 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Combat subject")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .build();
        let creature = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(creature, AttackTarget::Player(bob))],
        )
        .unwrap();
        assert!(combat_state::is_attacking(&combat, creature));
        game.combat = Some(combat);
        match case {
            0 => game.untap(creature),
            1 => {
                game.untap(creature);
                game.tap(creature);
            }
            2 | 3 | 4 => {
                let modification = match case {
                    2 => Modification::SetCardTypes(vec![CardType::Artifact]),
                    3 => Modification::ChangeController(bob),
                    _ => Modification::AddCardTypes(vec![CardType::Battle]),
                };
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(creature),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(creature, alice))
                .unwrap();
            }
            _ => {
                game.phase_out(creature);
            }
        }
        game.refresh_continuous_state().unwrap();
        results.push(combat_state::is_attacking(
            game.combat.as_ref().unwrap(),
            creature,
        ));
    }
    assert_eq!(
        results,
        vec![true, true, false, false, false, false],
        "CR 506.4/506.4b: untap, retap, lose creature, change controller, become battle, phase out"
    );
}

#[test]
fn cr_506_4_blocker_removal_keeps_the_attacker_blocked() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for case in 0..6 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Combat subject")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .build();
        let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let creature = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        combat_state::declare_blockers(&game, &mut combat, vec![(creature, attacker)]).unwrap();
        assert!(combat_state::is_blocking(&combat, creature));
        assert!(combat_state::is_blocked(&combat, attacker));
        game.combat = Some(combat);
        match case {
            0 => game.untap(creature),
            1 => {
                game.untap(creature);
                game.tap(creature);
            }
            2 | 3 | 4 => {
                let modification = match case {
                    2 => Modification::SetCardTypes(vec![CardType::Artifact]),
                    3 => Modification::ChangeController(alice),
                    _ => Modification::AddCardTypes(vec![CardType::Battle]),
                };
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(creature),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(creature, alice))
                .unwrap();
            }
            _ => {
                game.phase_out(creature);
            }
        }
        game.refresh_continuous_state().unwrap();
        assert!(
            combat_state::is_blocked(game.combat.as_ref().unwrap(), attacker),
            "CR 509.1h: losing the blocker does not make the attacker unblocked"
        );
        results.push(combat_state::is_blocking(
            game.combat.as_ref().unwrap(),
            creature,
        ));
        let mut runner = ironsmith::TurnRunner::from_state_for_sync(
            ironsmith::TurnRunnerState::CombatDamageRegular,
        );
        *runner.combat_mut() = game.combat.clone().unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..2 {
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                ironsmith::TurnAction::Continue
            ));
        }
        let expected_damage = if case < 2 { 3 } else { 0 };
        assert_eq!(
            game.damage_on(attacker),
            expected_damage,
            "CR 510.1d / 506.4b: tapped blocker still deals damage, removed blocker does not; case={case}"
        );
        assert_eq!(
            game.damage_on(creature),
            expected_damage,
            "CR 510.1c: attacker damages only a remaining blocker; case={case}"
        );
        assert_eq!(
            game.player(bob).unwrap().life,
            20,
            "CR 510.1c: a blocked attacker without trample cannot hit the defending player after losing its blocker"
        );
    }
    assert_eq!(
        results,
        vec![true, true, false, false, false, false],
        "CR 506.4/506.4b: untap, retap, lose creature, change controller, become battle, phase out"
    );
}

#[test]
fn cr_506_4c_removed_planeswalker_leaves_attacker_attacking_nothing() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{
        Ability, CardId, CardType, CounterType, Phase, StaticAbility, Step, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for case in 0..5 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let attacker_definition = CardDefinitionBuilder::new(CardId::new(), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .build();
        let attacker =
            game.create_object_from_definition(&attacker_definition, alice, Zone::Battlefield);
        let defender_definition =
            CardDefinitionBuilder::new(CardId::new(), "Defending planeswalker")
                .card_types(vec![CardType::Planeswalker])
                .loyalty(5)
                .build();
        let defender =
            game.create_object_from_definition(&defender_definition, bob, Zone::Battlefield);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Planeswalker(defender))],
        )
        .unwrap();
        game.combat = Some(combat);
        match case {
            0 => {}
            1 => {
                game.move_object_by_effect(defender, Zone::Exile);
            }
            2 => {
                game.phase_out(defender);
            }
            3 | 4 => {
                let modification = if case == 3 {
                    Modification::ChangeController(alice)
                } else {
                    Modification::SetCardTypes(vec![CardType::Artifact])
                };
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(defender),
                    modification,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
            }
            _ => unreachable!(),
        }
        game.refresh_continuous_state().unwrap();
        let mut combat = game.combat.take().unwrap();
        assert!(combat_state::is_attacking(&combat, attacker));
        assert_eq!(
            combat_state::get_attack_target(&combat, attacker)
                .unwrap()
                .is_nothing(),
            case != 0,
            "CR 506.4c case={case}"
        );
        game.turn.step = Some(Step::DeclareBlockers);
        combat_state::declare_blockers(&game, &mut combat, vec![]).unwrap();
        assert!(combat_state::is_unblocked(&combat, attacker));
        game.combat = Some(combat.clone());
        let mut runner = ironsmith::TurnRunner::from_state_for_sync(
            ironsmith::TurnRunnerState::CombatDamageRegular,
        );
        *runner.combat_mut() = combat;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..3 {
            let action = runner.advance(&mut game, &mut queue).unwrap();
            assert!(matches!(
                action,
                ironsmith::TurnAction::Continue | ironsmith::TurnAction::RunPriority
            ));
        }
        assert_eq!(
            game.player(bob).unwrap().life,
            20,
            "Damage never redirects to the former defender's controller"
        );
        assert_eq!(game.damage_on(attacker), 0);
        if case != 1 {
            assert_eq!(
                game.counter_count(defender, CounterType::Loyalty),
                if case == 0 { 2 } else { 5 },
                "CR 510.1b case={case}: only the still-attacked planeswalker takes damage"
            );
        }
    }
}

#[test]
fn cr_509_1g_blocker_remains_blocking_after_its_only_attacker_leaves() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Combat creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build();
    let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let blocker = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
    let mut combat = combat_state::new_combat();
    combat_state::declare_attackers(
        &mut game,
        &mut combat,
        vec![(attacker, AttackTarget::Player(bob))],
    )
    .unwrap();
    game.turn.step = Some(Step::DeclareBlockers);
    combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]).unwrap();
    assert!(combat_state::is_blocking(&combat, blocker));
    game.combat = Some(combat);
    game.move_object_by_effect(attacker, Zone::Exile).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.object(blocker).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.current_controller(blocker), Some(bob));
    assert!(!game.is_phased_out(blocker));
    assert!(
        combat_state::is_blocking(game.combat.as_ref().unwrap(), blocker),
        "CR 509.1g: a blocking creature remains blocking even when it is no longer blocking any attacking creature"
    );
}

#[test]
fn cr_510_1a_and_510_2_combat_damage_is_nonnegative_and_mutually_lethal_damage_is_simultaneous() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{
        Ability, CardId, CardType, Phase, StaticAbility, Step, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attacking_power in [-1, 0, 2, 3] {
        for blocking_power in [-1, 0, 2, 3] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let mut creatures = Vec::new();
            for (owner, power) in [(alice, attacking_power), (bob, blocking_power)] {
                let definition = CardDefinitionBuilder::new(CardId::new(), "Combat participant")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(power, 2))
                    .with_ability(Ability::static_ability(StaticAbility::haste()))
                    .with_ability(Ability::static_ability(StaticAbility::lifelink()))
                    .build();
                creatures.push(game.create_object_from_definition(
                    &definition,
                    owner,
                    Zone::Battlefield,
                ));
            }
            let (attacker, blocker) = (creatures[0], creatures[1]);
            let mut combat = combat_state::new_combat();
            combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(attacker, AttackTarget::Player(bob))],
            )
            .unwrap();
            game.turn.step = Some(Step::DeclareBlockers);
            combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]).unwrap();
            game.combat = Some(combat.clone());
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
            *runner.combat_mut() = combat;
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for _ in 0..2 {
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::Continue
                ));
            }
            assert_eq!(game.damage_on(attacker), blocking_power.max(0) as u32);
            assert_eq!(game.damage_on(blocker), attacking_power.max(0) as u32);
            assert_eq!(
                game.player(alice).unwrap().life,
                20 + attacking_power.max(0)
            );
            assert_eq!(game.player(bob).unwrap().life, 20 + blocking_power.max(0));
            assert!(game.stack.is_empty());
            assert_eq!(game.object(attacker).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(blocker).unwrap().zone, Zone::Battlefield);
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(game.battlefield.contains(&attacker), blocking_power < 2);
            assert_eq!(game.battlefield.contains(&blocker), attacking_power < 2);
            assert_eq!(
                game.player(alice).unwrap().graveyard.len(),
                usize::from(blocking_power >= 2)
            );
            assert_eq!(
                game.player(bob).unwrap().graveyard.len(),
                usize::from(attacking_power >= 2)
            );
        }
    }
}

#[test]
fn cr_510_4_first_and_double_strike_use_two_damage_steps_with_intervening_deaths() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{
        Ability, CardId, CardType, Phase, StaticAbility, Step, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attacking_kind in 0..3 {
        for blocking_kind in 0..3 {
            for toughness in [2, 7] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let mut creatures = Vec::new();
                for (owner, kind) in [(alice, attacking_kind), (bob, blocking_kind)] {
                    let mut builder =
                        CardDefinitionBuilder::new(CardId::new(), "Combat participant")
                            .card_types(vec![CardType::Creature])
                            .power_toughness(ironsmith::PowerToughness::fixed(3, toughness))
                            .with_ability(Ability::static_ability(StaticAbility::haste()))
                            .with_ability(Ability::static_ability(StaticAbility::lifelink()));
                    if kind == 1 {
                        builder = builder
                            .with_ability(Ability::static_ability(StaticAbility::first_strike()));
                    }
                    if kind == 2 {
                        builder = builder
                            .with_ability(Ability::static_ability(StaticAbility::double_strike()));
                    }
                    let definition = builder.build();
                    creatures.push(game.create_object_from_definition(
                        &definition,
                        owner,
                        Zone::Battlefield,
                    ));
                }
                let (attacker, blocker) = (creatures[0], creatures[1]);
                let mut combat = combat_state::new_combat();
                combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(attacker, AttackTarget::Player(bob))],
                )
                .unwrap();
                game.turn.step = Some(Step::DeclareBlockers);
                combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)])
                    .unwrap();
                game.combat = Some(combat.clone());
                let mut runner =
                    TurnRunner::from_state_for_sync(TurnRunnerState::DeclareBlockersPriority);
                *runner.combat_mut() = combat;
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut first_a_damage = 0;
                let mut first_b_damage = 0;
                if attacking_kind != 0 || blocking_kind != 0 {
                    first_a_damage = if attacking_kind != 0 { 3 } else { 0 };
                    first_b_damage = if blocking_kind != 0 { 3 } else { 0 };
                }
                let a_alive = first_b_damage < toughness;
                let b_alive = first_a_damage < toughness;
                let second_a_damage = if a_alive && b_alive && attacking_kind != 1 {
                    3
                } else {
                    0
                };
                let second_b_damage = if a_alive && b_alive && blocking_kind != 1 {
                    3
                } else {
                    0
                };
                for _ in 0..20 {
                    if matches!(runner.state(), TurnRunnerState::EndCombat) {
                        break;
                    }
                    assert!(matches!(
                        runner.advance(&mut game, &mut queue).unwrap(),
                        TurnAction::Continue | TurnAction::RunPriority
                    ));
                }
                assert!(matches!(runner.state(), TurnRunnerState::EndCombat));
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20 + first_a_damage + second_a_damage,
                    "attacker kind={attacking_kind}, blocker kind={blocking_kind}, toughness={toughness}"
                );
                assert_eq!(
                    game.player(bob).unwrap().life,
                    20 + first_b_damage + second_b_damage
                );
                assert_eq!(
                    game.battlefield.contains(&attacker),
                    first_b_damage + second_b_damage < toughness
                );
                assert_eq!(
                    game.battlefield.contains(&blocker),
                    first_a_damage + second_a_damage < toughness
                );
            }
        }
    }
}

#[test]
fn cr_702_4c_d_and_702_7c_strike_changes_between_damage_steps() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{
        Ability, CardId, CardType, Phase, StaticAbility, Step, TurnAction, TurnRunner,
        TurnRunnerState, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for initial in 0..3 {
        for final_kind in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let mut objects = Vec::new();
            for (owner, kind) in [(alice, initial), (bob, 1)] {
                let mut builder = CardDefinitionBuilder::new(CardId::new(), "Strike subject")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 10))
                    .with_ability(Ability::static_ability(StaticAbility::haste()));
                if kind > 0 {
                    builder = builder.with_ability(Ability::static_ability(if kind == 1 {
                        StaticAbility::first_strike()
                    } else {
                        StaticAbility::double_strike()
                    }));
                }
                objects.push(game.create_object_from_definition(
                    &builder.build(),
                    owner,
                    Zone::Battlefield,
                ));
            }
            let (attacker, blocker) = (objects[0], objects[1]);
            let mut combat = combat_state::new_combat();
            combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(attacker, AttackTarget::Player(bob))],
            )
            .unwrap();
            game.turn.step = Some(Step::DeclareBlockers);
            combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]).unwrap();
            game.combat = Some(combat.clone());
            let mut runner =
                TurnRunner::from_state_for_sync(TurnRunnerState::DeclareBlockersPriority);
            *runner.combat_mut() = combat;
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for _ in 0..10 {
                if matches!(
                    runner.state(),
                    TurnRunnerState::CombatDamageFirstStrikePriority
                ) {
                    break;
                }
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::Continue | TurnAction::RunPriority
                ));
            }
            assert!(matches!(
                runner.state(),
                TurnRunnerState::CombatDamageFirstStrikePriority
            ));
            assert_eq!(game.damage_on(blocker), if initial == 0 { 0 } else { 3 });
            ApplyContinuousEffect::new(
                EffectTarget::Specific(attacker),
                Modification::RemoveAllAbilities,
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
            .unwrap();
            if final_kind > 0 {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(attacker),
                    Modification::AddAbility(if final_kind == 1 {
                        StaticAbility::first_strike()
                    } else {
                        StaticAbility::double_strike()
                    }),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
            }
            for _ in 0..10 {
                if matches!(runner.state(), TurnRunnerState::EndCombat) {
                    break;
                }
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::Continue | TurnAction::RunPriority
                ));
            }
            assert!(matches!(runner.state(), TurnRunnerState::EndCombat));
            observed.push(game.damage_on(blocker));
            expected.push(if initial == 0 {
                3
            } else if final_kind == 2 {
                6
            } else {
                3
            });
            assert_eq!(game.damage_on(attacker), 3);
        }
    }
    assert_eq!(
        observed, expected,
        "Rows: initial normal/first/double strike; columns: final normal/first/double strike"
    );
}

#[test]
fn cr_702_9_and_702_17_flying_reach_legality_and_late_evasion_changes() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attacker_flying in 0..3 {
        for blocker_flying in 0..3 {
            for blocker_reach in 0..3 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let mut objects = Vec::new();
                for (owner, flying, reach) in [
                    (alice, attacker_flying, 0),
                    (bob, blocker_flying, blocker_reach),
                ] {
                    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Evasion subject")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                        .with_ability(Ability::static_ability(StaticAbility::haste()));
                    for _ in 0..flying {
                        builder =
                            builder.with_ability(Ability::static_ability(StaticAbility::flying()));
                    }
                    for _ in 0..reach {
                        builder =
                            builder.with_ability(Ability::static_ability(StaticAbility::reach()));
                    }
                    objects.push(game.create_object_from_definition(
                        &builder.build(),
                        owner,
                        Zone::Battlefield,
                    ));
                }
                let (attacker, blocker) = (objects[0], objects[1]);
                let mut combat = combat_state::new_combat();
                combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(attacker, AttackTarget::Player(bob))],
                )
                .unwrap();
                game.turn.step = Some(Step::DeclareBlockers);
                let result =
                    combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]);
                let legal = attacker_flying == 0 || blocker_flying > 0 || blocker_reach > 0;
                assert_eq!(
                    result.is_ok(),
                    legal,
                    "CR 702.9/702.17: attacker flying={attacker_flying}, blocker flying={blocker_flying}, reach={blocker_reach}"
                );
                if !legal {
                    continue;
                }
                game.combat = Some(combat);
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(blocker),
                    Modification::RemoveAllAbilities,
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(attacker),
                    Modification::AddAbility(StaticAbility::flying()),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
                game.refresh_continuous_state().unwrap();
                let combat = game.combat.as_ref().unwrap();
                assert!(
                    combat_state::is_blocking(combat, blocker),
                    "CR 509.1b: later evasion changes do not undo a legal block"
                );
                assert_eq!(combat_state::get_blockers(combat, attacker), &[blocker]);
            }
        }
    }
}

#[test]
fn cr_702_111_menace_counts_distinct_blockers_and_duplicate_abilities_are_redundant() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for menace_count in 0..3 {
        for blocker_count in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Menace attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .with_ability(Ability::static_ability(StaticAbility::haste()));
            for _ in 0..menace_count {
                builder = builder.with_ability(Ability::static_ability(StaticAbility::menace()));
            }
            let attacker =
                game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
            let blocker_definition = CardDefinitionBuilder::new(CardId::new(), "Blocker")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(1, 2))
                .build();
            let blockers: Vec<_> = (0..blocker_count)
                .map(|_| {
                    game.create_object_from_definition(&blocker_definition, bob, Zone::Battlefield)
                })
                .collect();
            let mut combat = combat_state::new_combat();
            combat_state::declare_attackers(
                &mut game,
                &mut combat,
                vec![(attacker, AttackTarget::Player(bob))],
            )
            .unwrap();
            game.turn.step = Some(Step::DeclareBlockers);
            let mut duplicate_combat = combat.clone();
            let result = combat_state::declare_blockers(
                &game,
                &mut combat,
                blockers.iter().map(|b| (*b, attacker)).collect(),
            );
            let legal = menace_count == 0 || blocker_count != 1;
            assert_eq!(
                result.is_ok(),
                legal,
                "CR 702.111: menace copies={menace_count}, blockers={blocker_count}"
            );
            if legal {
                assert_eq!(
                    combat_state::get_blockers(&combat, attacker).len(),
                    blocker_count
                );
                assert_eq!(
                    combat_state::is_blocked(&combat, attacker),
                    blocker_count > 0
                );
            }
            if let Some(blocker) = blockers.first() {
                assert!(
                    combat_state::declare_blockers(
                        &game,
                        &mut duplicate_combat,
                        vec![(*blocker, attacker), (*blocker, attacker)]
                    )
                    .is_err(),
                    "Repeating one creature in a declaration cannot satisfy menace"
                );
            }
        }
    }
}

#[test]
fn cr_702_118_skulk_compares_power_and_later_changes_preserve_blocks() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for attacker_power in [-1, 0, 2] {
        for blocker_power in [-1, 0, 2] {
            for skulk in 0..3 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let mut objects = Vec::new();
                for (owner, power, copies) in
                    [(alice, attacker_power, skulk), (bob, blocker_power, 0)]
                {
                    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Skulk subject")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(power, 4))
                        .with_ability(Ability::static_ability(StaticAbility::haste()));
                    for _ in 0..copies {
                        builder =
                            builder.with_ability(Ability::static_ability(StaticAbility::skulk()));
                    }
                    objects.push(game.create_object_from_definition(
                        &builder.build(),
                        owner,
                        Zone::Battlefield,
                    ));
                }
                let (attacker, blocker) = (objects[0], objects[1]);
                let mut combat = combat_state::new_combat();
                combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(attacker, AttackTarget::Player(bob))],
                )
                .unwrap();
                game.turn.step = Some(Step::DeclareBlockers);
                let result =
                    combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)]);
                let legal = skulk == 0 || blocker_power <= attacker_power;
                assert_eq!(
                    result.is_ok(),
                    legal,
                    "CR 702.118: attacker={attacker_power}, blocker={blocker_power}, skulk copies={skulk}"
                );
                if !legal {
                    continue;
                }
                game.combat = Some(combat);
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(blocker),
                    Modification::ModifyPowerToughness {
                        power: 10,
                        toughness: 0,
                    },
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(attacker),
                    Modification::AddAbility(StaticAbility::skulk()),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
                game.refresh_continuous_state().unwrap();
                let combat = game.combat.as_ref().unwrap();
                assert!(
                    combat_state::is_blocking(combat, blocker),
                    "CR 509.1b: later evasion changes do not undo a legal block"
                );
                assert_eq!(combat_state::get_blockers(combat, attacker), &[blocker]);
            }
        }
    }
}

#[test]
fn cr_702_3_and_702_20_defender_vigilance_and_tapped_attackers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for vigilance in 0..3 {
        for defender in 0..3 {
            for tapped in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let mut builder = CardDefinitionBuilder::new(CardId::new(), "Attack subject")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                    .with_ability(Ability::static_ability(StaticAbility::haste()));
                for _ in 0..vigilance {
                    builder =
                        builder.with_ability(Ability::static_ability(StaticAbility::vigilance()));
                }
                for _ in 0..defender {
                    builder =
                        builder.with_ability(Ability::static_ability(StaticAbility::defender()));
                }
                let creature =
                    game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
                if tapped {
                    game.tap(creature);
                }
                let mut combat = combat_state::new_combat();
                let result = combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(creature, AttackTarget::Player(bob))],
                );
                let legal = !tapped && defender == 0;
                assert_eq!(
                    result.is_ok(),
                    legal,
                    "CR 508.1a/702.3: vigilance={vigilance}, defender={defender}, tapped={tapped}"
                );
                assert_eq!(combat_state::is_attacking(&combat, creature), legal);
                assert_eq!(
                    game.is_tapped(creature),
                    if legal { vigilance == 0 } else { tapped },
                    "CR 508.1f/702.20: vigilance prevents attack tapping but does not allow a tapped attacker"
                );
            }
        }
    }
}

#[test]
fn cr_702_28_shadow_is_bidirectional_and_combines_with_flying() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for flying_attacker in [false, true] {
        for attacker_shadow in 0..3 {
            for blocker_shadow in 0..3 {
                for blocker_reach in 0..3 {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::DeclareAttackers);
                    let mut objects = Vec::new();
                    for (owner, shadow, reach) in [
                        (alice, attacker_shadow, 0),
                        (bob, blocker_shadow, blocker_reach),
                    ] {
                        let mut builder =
                            CardDefinitionBuilder::new(CardId::new(), "Evasion subject")
                                .card_types(vec![CardType::Creature])
                                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                                .with_ability(Ability::static_ability(StaticAbility::haste()));
                        for _ in 0..shadow {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::shadow()));
                        }
                        if owner == alice && flying_attacker {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::flying()));
                        }
                        for _ in 0..reach {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::reach()));
                        }
                        objects.push(game.create_object_from_definition(
                            &builder.build(),
                            owner,
                            Zone::Battlefield,
                        ));
                    }
                    let (attacker, blocker) = (objects[0], objects[1]);
                    let mut combat = combat_state::new_combat();
                    combat_state::declare_attackers(
                        &mut game,
                        &mut combat,
                        vec![(attacker, AttackTarget::Player(bob))],
                    )
                    .unwrap();
                    game.turn.step = Some(Step::DeclareBlockers);
                    let result = combat_state::declare_blockers(
                        &game,
                        &mut combat,
                        vec![(blocker, attacker)],
                    );
                    let legal = (attacker_shadow > 0) == (blocker_shadow > 0)
                        && (!flying_attacker || blocker_reach > 0);
                    assert_eq!(
                        result.is_ok(),
                        legal,
                        "CR 702.28/509.1b: attacker shadow={attacker_shadow}, blocker shadow={blocker_shadow}, flying attacker={flying_attacker}, blocker reach={blocker_reach}"
                    );
                    if !legal {
                        continue;
                    }
                    game.combat = Some(combat);
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(blocker),
                        Modification::RemoveAllAbilities,
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                    .unwrap();
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(attacker),
                        Modification::AddAbility(StaticAbility::shadow()),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                    .unwrap();
                    game.refresh_continuous_state().unwrap();
                    let combat = game.combat.as_ref().unwrap();
                    assert!(
                        combat_state::is_blocking(combat, blocker),
                        "CR 509.1b: later evasion changes do not undo a legal block"
                    );
                    assert_eq!(combat_state::get_blockers(combat, attacker), &[blocker]);
                }
            }
        }
    }
}

#[test]
fn cr_702_13_and_702_36_intimidate_and_fear_use_colors_and_artifact_type() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, CardId, CardType, ColorSet, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let colors = [
        ColorSet::WHITE,
        ColorSet::BLUE,
        ColorSet::BLACK,
        ColorSet::RED,
        ColorSet::GREEN,
    ];
    let color_set = |bits: u32| {
        colors
            .iter()
            .enumerate()
            .fold(ColorSet::default(), |set, (i, color)| {
                if bits & (1 << i) != 0 {
                    set.union(*color)
                } else {
                    set
                }
            })
    };
    for fear in [false, true] {
        for copies in [1, 2] {
            for attacker_colors in 0..32 {
                for blocker_colors in 0..32 {
                    for artifact in [false, true] {
                        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                        game.turn.active_player = alice;
                        game.turn.phase = Phase::Combat;
                        game.turn.step = Some(Step::DeclareAttackers);
                        let mut builder =
                            CardDefinitionBuilder::new(CardId::new(), "Colored attacker")
                                .card_types(vec![CardType::Creature])
                                .color_indicator(color_set(attacker_colors))
                                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                                .with_ability(Ability::static_ability(StaticAbility::haste()));
                        for _ in 0..copies {
                            builder = builder.with_ability(Ability::static_ability(if fear {
                                StaticAbility::fear()
                            } else {
                                StaticAbility::intimidate()
                            }));
                        }
                        let attacker = game.create_object_from_definition(
                            &builder.build(),
                            alice,
                            Zone::Battlefield,
                        );
                        let blocker_definition =
                            CardDefinitionBuilder::new(CardId::new(), "Colored blocker")
                                .card_types(if artifact {
                                    vec![CardType::Artifact, CardType::Creature]
                                } else {
                                    vec![CardType::Creature]
                                })
                                .color_indicator(color_set(blocker_colors))
                                .power_toughness(ironsmith::PowerToughness::fixed(1, 2))
                                .build();
                        let blocker = game.create_object_from_definition(
                            &blocker_definition,
                            bob,
                            Zone::Battlefield,
                        );
                        let mut combat = combat_state::new_combat();
                        combat_state::declare_attackers(
                            &mut game,
                            &mut combat,
                            vec![(attacker, AttackTarget::Player(bob))],
                        )
                        .unwrap();
                        game.turn.step = Some(Step::DeclareBlockers);
                        let legal = artifact
                            || if fear {
                                blocker_colors & 4 != 0
                            } else {
                                attacker_colors & blocker_colors != 0
                            };
                        assert_eq!(
                            combat_state::declare_blockers(
                                &game,
                                &mut combat,
                                vec![(blocker, attacker)]
                            )
                            .is_ok(),
                            legal,
                            "fear={fear}, copies={copies}, attacker colors={attacker_colors}, blocker colors={blocker_colors}, artifact={artifact}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cr_702_31_horsemanship_is_one_way_and_independent_of_flying() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Phase, StaticAbility, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for flying_attacker in [false, true] {
        for attacker_horsemanship in 0..3 {
            for blocker_horsemanship in 0..3 {
                for blocker_reach in 0..3 {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::DeclareAttackers);
                    let mut objects = Vec::new();
                    for (owner, horsemanship, reach) in [
                        (alice, attacker_horsemanship, 0),
                        (bob, blocker_horsemanship, blocker_reach),
                    ] {
                        let mut builder =
                            CardDefinitionBuilder::new(CardId::new(), "Evasion subject")
                                .card_types(vec![CardType::Creature])
                                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                                .with_ability(Ability::static_ability(StaticAbility::haste()));
                        for _ in 0..horsemanship {
                            builder = builder.with_ability(Ability::static_ability(
                                StaticAbility::horsemanship(),
                            ));
                        }
                        if owner == alice && flying_attacker {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::flying()));
                        }
                        for _ in 0..reach {
                            builder = builder
                                .with_ability(Ability::static_ability(StaticAbility::reach()));
                        }
                        objects.push(game.create_object_from_definition(
                            &builder.build(),
                            owner,
                            Zone::Battlefield,
                        ));
                    }
                    let (attacker, blocker) = (objects[0], objects[1]);
                    let mut combat = combat_state::new_combat();
                    combat_state::declare_attackers(
                        &mut game,
                        &mut combat,
                        vec![(attacker, AttackTarget::Player(bob))],
                    )
                    .unwrap();
                    game.turn.step = Some(Step::DeclareBlockers);
                    let result = combat_state::declare_blockers(
                        &game,
                        &mut combat,
                        vec![(blocker, attacker)],
                    );
                    let legal = (attacker_horsemanship == 0 || blocker_horsemanship > 0)
                        && (!flying_attacker || blocker_reach > 0);
                    assert_eq!(
                        result.is_ok(),
                        legal,
                        "CR 702.31/509.1b: attacker horsemanship={attacker_horsemanship}, blocker horsemanship={blocker_horsemanship}, flying attacker={flying_attacker}, blocker reach={blocker_reach}"
                    );
                    if !legal {
                        continue;
                    }
                    game.combat = Some(combat);
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(blocker),
                        Modification::RemoveAllAbilities,
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                    .unwrap();
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(attacker),
                        Modification::AddAbility(StaticAbility::horsemanship()),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                    .unwrap();
                    game.refresh_continuous_state().unwrap();
                    let combat = game.combat.as_ref().unwrap();
                    assert!(
                        combat_state::is_blocking(combat, blocker),
                        "CR 509.1b: later evasion changes do not undo a legal block"
                    );
                    assert_eq!(combat_state::get_blockers(combat, attacker), &[blocker]);
                }
            }
        }
    }
}

#[test]
fn cr_702_14_landwalk_uses_defending_players_matching_lands() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{
        Ability, CardId, CardType, Phase, StaticAbility, Step, Subtype, Supertype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for kind in 0..7 {
        for land_kind in 0..6 {
            for land_owner in [alice, bob] {
                for copies in [1, 2] {
                    for blocker_walk in [false, true] {
                        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                        game.turn.active_player = alice;
                        game.turn.phase = Phase::Combat;
                        game.turn.step = Some(Step::DeclareAttackers);
                        let walk = match kind {
                            0 => StaticAbility::landwalk(Subtype::Island),
                            1 => StaticAbility::snow_landwalk(Subtype::Forest),
                            2 => StaticAbility::nonbasic_landwalk(),
                            3 => StaticAbility::artifact_landwalk(),
                            4 => StaticAbility::legendary_landwalk(),
                            5 => StaticAbility::snow_any_landwalk(),
                            _ => StaticAbility::any_landwalk(),
                        };
                        let mut objects = Vec::new();
                        for (owner, count) in [(alice, copies), (bob, usize::from(blocker_walk))] {
                            let mut builder =
                                CardDefinitionBuilder::new(CardId::new(), "Landwalker")
                                    .card_types(vec![CardType::Creature])
                                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                                    .with_ability(Ability::static_ability(StaticAbility::haste()));
                            for _ in 0..count {
                                builder =
                                    builder.with_ability(Ability::static_ability(walk.clone()));
                            }
                            objects.push(game.create_object_from_definition(
                                &builder.build(),
                                owner,
                                Zone::Battlefield,
                            ));
                        }
                        let mut land = CardDefinitionBuilder::new(CardId::new(), "Terrain")
                            .card_types(match land_kind {
                                3 => vec![CardType::Artifact, CardType::Land],
                                5 => vec![CardType::Artifact],
                                _ => vec![CardType::Land],
                            });
                        if land_kind <= 2 {
                            land = land
                                .subtypes(vec![if land_kind == 0 {
                                    Subtype::Island
                                } else {
                                    Subtype::Forest
                                }])
                                .supertypes(if land_kind == 2 {
                                    vec![Supertype::Basic, Supertype::Snow]
                                } else {
                                    vec![Supertype::Basic]
                                });
                        }
                        if land_kind == 4 {
                            land = land.supertypes(vec![Supertype::Legendary]);
                        }
                        game.create_object_from_definition(
                            &land.build(),
                            land_owner,
                            Zone::Battlefield,
                        );
                        let (attacker, blocker) = (objects[0], objects[1]);
                        let mut combat = combat_state::new_combat();
                        combat_state::declare_attackers(
                            &mut game,
                            &mut combat,
                            vec![(attacker, AttackTarget::Player(bob))],
                        )
                        .unwrap();
                        game.turn.step = Some(Step::DeclareBlockers);
                        let matching = match kind {
                            0 => land_kind == 0,
                            1 | 5 => land_kind == 2,
                            2 => land_kind == 3 || land_kind == 4,
                            3 => land_kind == 3,
                            4 => land_kind == 4,
                            _ => land_kind != 5,
                        };
                        let legal = land_owner != bob || !matching;
                        assert_eq!(
                            combat_state::declare_blockers(
                                &game,
                                &mut combat,
                                vec![(blocker, attacker)]
                            )
                            .is_ok(),
                            legal,
                            "CR 702.14: walk={kind}, terrain={land_kind}, terrain owner={land_owner:?}, copies={copies}, blocker walk={blocker_walk}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cr_702_16f_protection_blocks_matching_creatures() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Ability, CardId, CardType, ColorSet, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let colors = [
        ColorSet::WHITE,
        ColorSet::BLUE,
        ColorSet::BLACK,
        ColorSet::RED,
        ColorSet::GREEN,
    ];
    let color_set = |bits: u32| {
        colors
            .iter()
            .enumerate()
            .fold(ColorSet::default(), |set, (i, color)| {
                if bits & (1 << i) != 0 {
                    set.union(*color)
                } else {
                    set
                }
            })
    };
    for protection_kind in 0..37 {
        for blocker_colors in 0..32 {
            for artifact in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let mut builder = CardDefinitionBuilder::new(CardId::new(), "Colored attacker")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                    .with_ability(Ability::static_ability(StaticAbility::haste()));
                use ironsmith::ability::ProtectionFrom;
                let protection = match protection_kind {
                    0..32 => ProtectionFrom::Color(color_set(protection_kind)),
                    32 => ProtectionFrom::Colorless,
                    33 => ProtectionFrom::AllColors,
                    34 => ProtectionFrom::Creatures,
                    35 => ProtectionFrom::CardType(CardType::Artifact),
                    _ => ProtectionFrom::Everything,
                };
                builder = builder.with_ability(Ability::static_ability(StaticAbility::protection(
                    protection,
                )));
                let attacker =
                    game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
                let blocker_definition =
                    CardDefinitionBuilder::new(CardId::new(), "Colored blocker")
                        .card_types(if artifact {
                            vec![CardType::Artifact, CardType::Creature]
                        } else {
                            vec![CardType::Creature]
                        })
                        .color_indicator(color_set(blocker_colors))
                        .power_toughness(ironsmith::PowerToughness::fixed(1, 2))
                        .build();
                let blocker =
                    game.create_object_from_definition(&blocker_definition, bob, Zone::Battlefield);
                let mut combat = combat_state::new_combat();
                combat_state::declare_attackers(
                    &mut game,
                    &mut combat,
                    vec![(attacker, AttackTarget::Player(bob))],
                )
                .unwrap();
                game.turn.step = Some(Step::DeclareBlockers);
                let prohibited = match protection_kind {
                    0..32 => protection_kind & blocker_colors != 0,
                    32 => blocker_colors == 0,
                    33 => blocker_colors != 0,
                    34 | 36 => true,
                    _ => artifact,
                };
                let legal = !prohibited;
                assert_eq!(
                    combat_state::declare_blockers(&game, &mut combat, vec![(blocker, attacker)])
                        .is_ok(),
                    legal,
                    "protection={protection_kind}, blocker colors={blocker_colors}, artifact={artifact}"
                );
            }
        }
    }
}

#[test]
fn cr_702_16e_protection_prevents_damage_except_when_prevention_is_forbidden() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, ColorSet, Phase, StaticAbility, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let colors = [
        ColorSet::WHITE,
        ColorSet::BLUE,
        ColorSet::BLACK,
        ColorSet::RED,
        ColorSet::GREEN,
    ];
    let color_set = |bits: u32| {
        colors
            .iter()
            .enumerate()
            .fold(ColorSet::default(), |set, (i, color)| {
                if bits & (1 << i) != 0 {
                    set.union(*color)
                } else {
                    set
                }
            })
    };
    for unpreventable in [false, true] {
        for protection_kind in [1, 2, 4, 8, 16, 31, 32, 33, 34, 35, 36] {
            for blocker_colors in 0..32 {
                for artifact in [false, true] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.phase = Phase::Combat;
                    game.turn.step = Some(Step::DeclareAttackers);
                    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Colored attacker")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                        .with_ability(Ability::static_ability(StaticAbility::haste()));
                    use ironsmith::ability::ProtectionFrom;
                    let protection = match protection_kind {
                        0..32 => ProtectionFrom::Color(color_set(protection_kind)),
                        32 => ProtectionFrom::Colorless,
                        33 => ProtectionFrom::AllColors,
                        34 => ProtectionFrom::Creatures,
                        35 => ProtectionFrom::CardType(CardType::Artifact),
                        _ => ProtectionFrom::Everything,
                    };
                    builder = builder.with_ability(Ability::static_ability(
                        StaticAbility::protection(protection),
                    ));
                    let attacker = game.create_object_from_definition(
                        &builder.build(),
                        alice,
                        Zone::Battlefield,
                    );
                    let mut source_builder =
                        CardDefinitionBuilder::new(CardId::new(), "Colored blocker")
                            .card_types(if artifact {
                                vec![CardType::Artifact, CardType::Creature]
                            } else {
                                vec![CardType::Creature]
                            })
                            .color_indicator(color_set(blocker_colors))
                            .power_toughness(ironsmith::PowerToughness::fixed(1, 2))
                            .with_ability(Ability::static_ability(StaticAbility::lifelink()));
                    if unpreventable {
                        source_builder = source_builder.with_ability(Ability::static_ability(
                            StaticAbility::damage_cant_be_prevented(),
                        ));
                    }
                    let blocker_definition = source_builder.build();
                    let blocker = game.create_object_from_definition(
                        &blocker_definition,
                        bob,
                        Zone::Battlefield,
                    );
                    let prohibited = match protection_kind {
                        0..32 => protection_kind & blocker_colors != 0,
                        32 => blocker_colors == 0,
                        33 => blocker_colors != 0,
                        34 | 36 => true,
                        _ => artifact,
                    };
                    ironsmith::effects::DealDamageEffect::new(
                        3,
                        ironsmith::ChooseSpec::SpecificObject(attacker),
                    )
                    .execute(&mut game, &mut EffectContext::new_default(blocker, bob))
                    .unwrap();
                    let dealt = if prohibited && !unpreventable { 0 } else { 3 };
                    assert_eq!(
                        game.damage_on(attacker),
                        dealt,
                        "CR 702.16e: protection={protection_kind}, source colors={blocker_colors}, artifact={artifact}, unpreventable={unpreventable}"
                    );
                    assert_eq!(
                        game.player(bob).unwrap().life,
                        20 + dealt as i32,
                        "Only dealt damage causes lifelink gain"
                    );
                }
            }
        }
    }
}

#[test]
fn cr_702_16c_d_protection_removes_matching_attachments_via_state_based_actions() {
    use ironsmith::ability::ProtectionFrom;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{
        CardId, CardType, ColorSet, ObjectFilter, StaticAbility, Subtype, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut wrong_attachments = Vec::new();
    for kind in 0..3 {
        for color in 0..3 {
            for protected in 0..4 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let host_def = CardDefinitionBuilder::new(CardId::new(), "Creature land host")
                    .card_types(vec![CardType::Creature, CardType::Land])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                    .build();
                let host = game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
                let mut builder = CardDefinitionBuilder::new(CardId::new(), "Attachment")
                    .card_types(vec![if kind == 0 {
                        CardType::Enchantment
                    } else {
                        CardType::Artifact
                    }])
                    .subtypes(vec![match kind {
                        0 => Subtype::Aura,
                        1 => Subtype::Equipment,
                        _ => Subtype::Fortification,
                    }])
                    .color_indicator(match color {
                        0 => ColorSet::RED,
                        1 => ColorSet::BLUE,
                        _ => ColorSet::default(),
                    });
                if kind == 0 {
                    builder = builder.enchants(ObjectFilter::creature());
                }
                let attachment =
                    game.create_object_from_definition(&builder.build(), bob, Zone::Battlefield);
                assert!(game.attach_object_to_target(attachment, AttachmentTarget::Object(host)));
                let quality = match protected {
                    0 => ProtectionFrom::Color(ColorSet::RED),
                    1 => ProtectionFrom::Color(ColorSet::BLUE),
                    2 => ProtectionFrom::CardType(CardType::Artifact),
                    _ => ProtectionFrom::Everything,
                };
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(host),
                    Modification::AddAbility(StaticAbility::protection(quality)),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(host, alice))
                .unwrap();
                let matches = match protected {
                    0 => color == 0,
                    1 => color == 1,
                    2 => kind != 0,
                    _ => true,
                };
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                if matches && kind == 0 {
                    assert!(!game.battlefield.contains(&attachment));
                    assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
                    assert!(game.player(alice).unwrap().graveyard.is_empty());
                } else {
                    assert!(game.battlefield.contains(&attachment));
                    let expected = if matches {
                        None
                    } else {
                        Some(AttachmentTarget::Object(host))
                    };
                    if game.object(attachment).unwrap().attached_to != expected {
                        wrong_attachments.push((kind, color, protected));
                    }
                    assert!(game.player(bob).unwrap().graveyard.is_empty());
                }
                assert!(game.battlefield.contains(&host));
            }
        }
    }
    assert!(
        wrong_attachments.is_empty(),
        "CR 702.16c-d: incorrect attachments (kind,color,protection): {wrong_attachments:?}"
    );
}

#[test]
fn cr_702_16c_d_protection_rejects_reattachment_and_preserves_old_host() {
    use ironsmith::ability::ProtectionFrom;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{
        CardId, CardType, ColorSet, ObjectFilter, StaticAbility, Subtype, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut wrong_attachments = Vec::new();
    for plural in [false, true] {
        for kind in 0..3 {
            for color in 0..3 {
                for protected in 0..4 {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let host_def = CardDefinitionBuilder::new(CardId::new(), "Creature land host")
                        .card_types(vec![CardType::Creature, CardType::Land])
                        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                        .build();
                    let host =
                        game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
                    let old_host =
                        game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
                    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Attachment")
                        .card_types(vec![if kind == 0 {
                            CardType::Enchantment
                        } else {
                            CardType::Artifact
                        }])
                        .subtypes(vec![match kind {
                            0 => Subtype::Aura,
                            1 => Subtype::Equipment,
                            _ => Subtype::Fortification,
                        }])
                        .color_indicator(match color {
                            0 => ColorSet::RED,
                            1 => ColorSet::BLUE,
                            _ => ColorSet::default(),
                        });
                    if kind == 0 {
                        builder = builder.enchants(ObjectFilter::creature());
                    }
                    let attachment = game.create_object_from_definition(
                        &builder.build(),
                        bob,
                        Zone::Battlefield,
                    );
                    assert!(
                        game.attach_object_to_target(
                            attachment,
                            AttachmentTarget::Object(old_host)
                        )
                    );
                    let quality = match protected {
                        0 => ProtectionFrom::Color(ColorSet::RED),
                        1 => ProtectionFrom::Color(ColorSet::BLUE),
                        2 => ProtectionFrom::CardType(CardType::Artifact),
                        _ => ProtectionFrom::Everything,
                    };
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(host),
                        Modification::AddAbility(StaticAbility::protection(quality)),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(host, alice))
                    .unwrap();
                    let matches = match protected {
                        0 => color == 0,
                        1 => color == 1,
                        2 => kind != 0,
                        _ => true,
                    };
                    if plural {
                        ironsmith::effects::AttachObjectsEffect::new(
                            ironsmith::ChooseSpec::SpecificObject(attachment),
                            ironsmith::ChooseSpec::SpecificObject(host),
                        )
                        .execute(&mut game, &mut EffectContext::new_default(attachment, bob))
                        .unwrap();
                    } else {
                        ironsmith::effects::AttachToEffect::new(
                            ironsmith::ChooseSpec::SpecificObject(host),
                        )
                        .execute(&mut game, &mut EffectContext::new_default(attachment, bob))
                        .unwrap();
                    }
                    let expected = Some(AttachmentTarget::Object(if matches {
                        old_host
                    } else {
                        host
                    }));
                    if game.object(attachment).unwrap().attached_to != expected {
                        wrong_attachments.push((plural, kind, color, protected));
                    }
                    assert!(game.battlefield.contains(&attachment));
                    assert!(game.player(bob).unwrap().graveyard.is_empty());
                    assert!(game.battlefield.contains(&host));
                }
            }
        }
    }
    assert!(
        wrong_attachments.is_empty(),
        "CR 702.16c-d: incorrect reattachments (plural,kind,color,protection): {wrong_attachments:?}"
    );
}

#[test]
fn cr_701_3b_same_host_and_nonattachment_objects_do_not_attach_again() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{AttachObjectsEffect, AttachToEffect};
    use ironsmith::events::{ObjectBecameAttachedEvent, ObjectBecameUnattachedEvent};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{CardId, CardType, ChooseSpec, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    for plural in [false, true] {
        for kind in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host_def = CardDefinitionBuilder::new(CardId::new(), "Host")
                .card_types(vec![CardType::Creature, CardType::Land])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
            let host = game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Attachment subject")
                .card_types(vec![if kind == 0 {
                    CardType::Enchantment
                } else {
                    CardType::Artifact
                }]);
            if kind < 3 {
                builder = builder.subtypes(vec![match kind {
                    0 => Subtype::Aura,
                    1 => Subtype::Equipment,
                    _ => Subtype::Fortification,
                }]);
            }
            if kind == 0 {
                builder = builder.enchants(ObjectFilter::creature());
            }
            let source =
                game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
            if kind < 3 {
                assert!(game.attach_object_to_target(source, AttachmentTarget::Object(host)));
            }
            let initial_timestamp = game
                .effect_store
                .continuous_effects
                .get_object_timestamp(source)
                .unwrap();
            let entry_timestamp = game
                .effect_store
                .continuous_effects
                .get_entry_timestamp(source);
            game.take_pending_trigger_events();
            let mut ctx = EffectContext::new_default(source, alice);
            let outcome = if plural {
                AttachObjectsEffect::new(
                    ChooseSpec::SpecificObject(source),
                    ChooseSpec::SpecificObject(host),
                )
                .execute(&mut game, &mut ctx)
                .unwrap()
            } else {
                AttachToEffect::new(ChooseSpec::SpecificObject(host))
                    .execute(&mut game, &mut ctx)
                    .unwrap()
            };
            let mut events = game.take_pending_trigger_events();
            events.extend(outcome.events);
            assert!(
                events.iter().all(
                    |event| event.downcast::<ObjectBecameAttachedEvent>().is_none()
                        && event.downcast::<ObjectBecameUnattachedEvent>().is_none()
                ),
                "CR 701.3b: no attachment transition for plural={plural}, kind={kind}"
            );
            assert_eq!(
                game.object(source).unwrap().attached_to,
                if kind < 3 {
                    Some(AttachmentTarget::Object(host))
                } else {
                    None
                }
            );
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.battlefield.len(), 2);
            assert_eq!(
                game.effect_store
                    .continuous_effects
                    .get_object_timestamp(source),
                Some(initial_timestamp),
                "CR 701.3b: no-op attachment does not reset timestamp"
            );
            if kind < 3 {
                let new_host =
                    game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
                if plural {
                    AttachObjectsEffect::new(
                        ChooseSpec::SpecificObject(source),
                        ChooseSpec::SpecificObject(new_host),
                    )
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                } else {
                    AttachToEffect::new(ChooseSpec::SpecificObject(new_host))
                        .execute(&mut game, &mut ctx)
                        .unwrap();
                }
                assert_eq!(
                    game.object(source).unwrap().attached_to,
                    Some(AttachmentTarget::Object(new_host))
                );
                assert!(
                    game.effect_store
                        .continuous_effects
                        .get_object_timestamp(source)
                        .unwrap()
                        > initial_timestamp,
                    "CR 701.3c / 613.7e: a different attachment destination gives a new timestamp"
                );
                assert_eq!(
                    game.effect_store
                        .continuous_effects
                        .get_entry_timestamp(source),
                    entry_timestamp,
                    "Reattachment is not zone entry"
                );
            }
        }
    }
}

#[test]
fn cr_613_7f_and_708_2b_face_changes_refresh_timestamp_without_zone_change() {
    use ironsmith::effects::{TurnFaceDownEffect, TurnFaceUpEffect};
    use ironsmith::{ChooseSpec, CounterType, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = counter_creature(&mut game, Zone::Battlefield);
    game.add_counters(creature, CounterType::PlusOnePlusOne, 1);
    game.tap(creature);
    let stable_id = game.object(creature).unwrap().stable_id;
    let initial = game
        .effect_store
        .continuous_effects
        .get_object_timestamp(creature)
        .unwrap();
    let mut ctx = EffectContext::new_default(creature, alice);
    TurnFaceDownEffect::new(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(game.is_face_down(creature));
    assert_eq!(
        (
            game.current_power(creature),
            game.current_toughness(creature)
        ),
        (Some(3), Some(3))
    );
    let down = game
        .effect_store
        .continuous_effects
        .get_object_timestamp(creature)
        .unwrap();
    assert!(
        down > initial,
        "CR 613.7f: turning face down gives a new timestamp"
    );
    TurnFaceDownEffect::new(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.effect_store
            .continuous_effects
            .get_object_timestamp(creature),
        Some(down),
        "CR 708.2b: a second face-down instruction does nothing"
    );
    TurnFaceUpEffect::new(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(!game.is_face_down(creature));
    let up = game
        .effect_store
        .continuous_effects
        .get_object_timestamp(creature)
        .unwrap();
    assert!(
        up > down,
        "CR 613.7f: turning face up gives a new timestamp"
    );
    assert_eq!(
        (
            game.current_power(creature),
            game.current_toughness(creature)
        ),
        (Some(4), Some(5))
    );
    TurnFaceUpEffect::new(ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        game.effect_store
            .continuous_effects
            .get_object_timestamp(creature),
        Some(up)
    );
    assert_eq!(game.object(creature).unwrap().stable_id, stable_id);
    assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.counter_count(creature, CounterType::PlusOnePlusOne), 1);
    assert!(game.is_tapped(creature));
    assert_eq!(game.battlefield.len(), 1);
}

#[test]
fn cr_708_2a_and_708_8_face_down_characteristics_replace_and_restore_printed_values() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{TurnFaceDownEffect, TurnFaceUpEffect};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ColorSet, ManaCost, ManaSymbol, StaticAbility,
        Subtype, Supertype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mana_cost = ManaCost::from_symbols(vec![ManaSymbol::Red, ManaSymbol::Generic(3)]);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Printed identity")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Goblin])
        .supertypes(vec![Supertype::Legendary])
        .color_indicator(ColorSet::RED.union(ColorSet::BLUE))
        .mana_cost(mana_cost.clone())
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(StaticAbility::flying()))
        .build();
    let object = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(object, alice);
    TurnFaceDownEffect::new(ChooseSpec::SpecificObject(object))
        .execute(&mut game, &mut ctx)
        .unwrap();
    let down = game.current_characteristics(object).unwrap();
    assert_eq!(&*down.card_types, &[CardType::Creature]);
    assert!(down.subtypes.is_empty());
    assert!(down.supertypes.is_empty());
    assert!(down.colors.is_empty());
    assert!(down.mana_cost.is_none());
    assert!(down.abilities.is_empty());
    assert_eq!((down.power, down.toughness), (Some(2), Some(2)));
    TurnFaceUpEffect::new(ChooseSpec::SpecificObject(object))
        .execute(&mut game, &mut ctx)
        .unwrap();
    let up = game.current_characteristics(object).unwrap();
    assert!(up.card_types.contains(&CardType::Artifact));
    assert!(up.card_types.contains(&CardType::Creature));
    assert_eq!(&*up.subtypes, &[Subtype::Goblin]);
    assert_eq!(&*up.supertypes, &[Supertype::Legendary]);
    assert_eq!(up.colors, ColorSet::RED.union(ColorSet::BLUE));
    assert_eq!(up.mana_cost, Some(mana_cost));
    assert_eq!((up.power, up.toughness), (Some(3), Some(4)));
    assert!(game.object_has_ability(object, &StaticAbility::flying()));
    assert_eq!(
        game.current_name(object).as_deref(),
        Some("Printed identity")
    );
}

#[test]
fn cr_708_10_face_down_copy_keeps_overlay_until_turned_face_up() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{
        ApplyContinuousEffect, RuntimeModification, TurnFaceDownEffect, TurnFaceUpEffect,
    };
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ColorSet, CounterType, StaticAbility, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let donor_definition = CardDefinitionBuilder::new(CardId::new(), "Copied identity")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .color_indicator(ColorSet::GREEN)
        .power_toughness(ironsmith::PowerToughness::fixed(4, 7))
        .with_ability(Ability::static_ability(StaticAbility::vigilance()))
        .build();
    let donor = game.create_object_from_definition(&donor_definition, alice, Zone::Battlefield);
    game.add_counters(donor, CounterType::PlusOnePlusOne, 1);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(donor),
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(donor, alice))
    .unwrap();
    assert_eq!(
        (game.current_power(donor), game.current_toughness(donor)),
        (Some(7), Some(10))
    );
    let recipient = counter_creature(&mut game, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(recipient, alice);
    TurnFaceDownEffect::new(ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(recipient),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(donor),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(game.is_face_down(recipient));
    assert_eq!(
        (
            game.current_power(recipient),
            game.current_toughness(recipient)
        ),
        (Some(2), Some(2))
    );
    assert_eq!(
        &*game.current_card_types(recipient).unwrap(),
        &[CardType::Creature]
    );
    assert!(game.current_colors(recipient).unwrap().is_empty());
    assert!(!game.object_has_ability(recipient, &StaticAbility::vigilance()));
    let hidden_copy = counter_creature(&mut game, Zone::Battlefield);
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(hidden_copy),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(recipient),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert!(
        !game.is_face_down(hidden_copy),
        "Copying face-down values does not copy face-down status"
    );
    assert_eq!(
        (
            game.current_power(hidden_copy),
            game.current_toughness(hidden_copy)
        ),
        (Some(2), Some(2))
    );
    TurnFaceUpEffect::new(ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(!game.is_face_down(recipient));
    assert_eq!(
        game.current_name(recipient).as_deref(),
        Some("Copied identity")
    );
    assert_eq!(
        (
            game.current_power(recipient),
            game.current_toughness(recipient)
        ),
        (Some(4), Some(7))
    );
    assert!(
        game.current_card_types(recipient)
            .unwrap()
            .contains(&CardType::Artifact)
    );
    assert_eq!(game.current_colors(recipient), Some(ColorSet::GREEN));
    assert!(game.object_has_ability(recipient, &StaticAbility::vigilance()));
    assert_eq!(
        game.counter_count(recipient, CounterType::PlusOnePlusOne),
        0
    );
    let visible_copy = counter_creature(&mut game, Zone::Battlefield);
    ApplyContinuousEffect::new_runtime(
        EffectTarget::Specific(visible_copy),
        RuntimeModification::CopyOf {
            source: ChooseSpec::SpecificObject(recipient),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: vec![],
            copy_exception_surface: None,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    game.move_object_by_effect(donor, Zone::Exile).unwrap();
    for copy in [recipient, visible_copy] {
        assert_eq!(
            (game.current_power(copy), game.current_toughness(copy)),
            (Some(4), Some(7))
        );
        assert_eq!(game.current_colors(copy), Some(ColorSet::GREEN));
        assert!(game.object_has_ability(copy, &StaticAbility::vigilance()));
    }
    assert_eq!(
        (
            game.current_power(hidden_copy),
            game.current_toughness(hidden_copy)
        ),
        (Some(2), Some(2)),
        "CR 707.2b: turning the source face up does not change a previous copy"
    );
    assert!(game.current_colors(hidden_copy).unwrap().is_empty());
    assert!(!game.object_has_ability(hidden_copy, &StaticAbility::vigilance()));
    ironsmith::turn::execute_cleanup_step(&mut game);
    for copy in [recipient, hidden_copy, visible_copy] {
        assert_eq!(
            (game.current_power(copy), game.current_toughness(copy)),
            (Some(3), Some(4)),
            "Copy effect expiration restores the underlying object's values"
        );
        assert_eq!(
            game.current_name(copy).as_deref(),
            Some("Counter recipient")
        );
    }
}

#[test]
fn cr_707_4_changing_copies_preserves_noncopy_state_and_does_not_reenter() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect, RuntimeModification};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, CounterType, Effect, StaticAbility, Subtype, Until,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let recipient = counter_creature(&mut game, Zone::Battlefield);
    let stable = game.object(recipient).unwrap().stable_id;
    game.add_counters(recipient, CounterType::PlusOnePlusOne, 1);
    game.tap(recipient);
    let equipment_def = CardDefinitionBuilder::new(CardId::new(), "Attached Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment = game.create_object_from_definition(&equipment_def, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(recipient)));
    let mut ctx = EffectContext::new_default(recipient, alice);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(recipient),
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    ApplyContinuousEffect::new(
        EffectTarget::Specific(recipient),
        Modification::AddAbility(StaticAbility::flying()),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(recipient))
        .execute(&mut game, &mut ctx)
        .unwrap();
    for (name, power, toughness) in [("First donor", 3, 6), ("Second donor", 5, 7)] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(power, toughness))
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let donor = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        game.take_pending_trigger_events();
        ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(recipient),
            RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(donor),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: vec![],
                copy_exception_surface: None,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert!(
            queue.entries.is_empty(),
            "CR 707.4: gaining ETB ability while copying is not entering"
        );
        assert_eq!(game.current_name(recipient).as_deref(), Some(name));
        assert_eq!(
            (
                game.current_power(recipient),
                game.current_toughness(recipient)
            ),
            (Some(power + 3), Some(toughness + 3))
        );
        assert_eq!(game.current_controller(recipient), Some(alice));
        assert_eq!(game.object(recipient).unwrap().owner, alice);
        assert_eq!(game.object(recipient).unwrap().stable_id, stable);
        assert_eq!(game.object(recipient).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.counter_count(recipient, CounterType::PlusOnePlusOne),
            1
        );
        assert_eq!(game.damage_on(recipient), 1);
        assert!(game.is_tapped(recipient));
        assert!(game.object_has_ability(recipient, &StaticAbility::flying()));
        assert_eq!(
            game.object(equipment).unwrap().attached_to,
            Some(AttachmentTarget::Object(recipient))
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn cr_707_5_created_copies_enter_with_copied_counters_and_entry_triggers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::CreateTokenCopyEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, CounterType, Effect, Zone};
    let alice = PlayerId::from_index(0);
    for kind in [CardType::Creature, CardType::Planeswalker, CardType::Battle] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Copied entry subject")
            .card_types(vec![kind])
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(1)],
            ));
        builder = match kind {
            CardType::Planeswalker => builder.loyalty(5),
            CardType::Battle => builder.defense(7),
            _ => builder.power_toughness(ironsmith::PowerToughness::fixed(3, 4)),
        };
        let donor = game.create_object_from_definition(&builder.build(), alice, Zone::Graveyard);
        game.take_pending_trigger_events();
        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(donor))
            .execute(&mut game, &mut EffectContext::new_default(donor, alice))
            .unwrap();
        assert_eq!(game.battlefield.len(), 1);
        let copy = game.battlefield[0];
        assert_eq!(
            game.object(copy).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        );
        assert_eq!(
            game.current_name(copy).as_deref(),
            Some("Copied entry subject")
        );
        assert!(game.current_card_types(copy).unwrap().contains(&kind));
        match kind {
            CardType::Planeswalker => assert_eq!(game.counter_count(copy, CounterType::Loyalty), 5),
            CardType::Battle => assert_eq!(game.counter_count(copy, CounterType::Defense), 7),
            _ => assert_eq!(
                (game.current_power(copy), game.current_toughness(copy)),
                (Some(3), Some(4))
            ),
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert_eq!(
            queue.entries.len(),
            1,
            "CR 707.5: copied ETB ability triggers once for {kind:?}"
        );
        assert_eq!(queue.entries[0].source, copy);
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "Entry trigger has not resolved yet"
        );
    }
}

#[test]
fn cr_707_6_copying_protection_does_not_copy_originals_chosen_color() {
    use ironsmith::ability::ProtectionFrom;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::EffectTarget;
    use ironsmith::effects::{
        ApplyContinuousEffect, CreateTokenCopyEffect, DealDamageEffect, RuntimeModification,
    };
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Color, ColorSet, StaticAbility, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    for prior_choice in [None, Some(Color::Blue)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Chosen-color protection")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 10))
            .with_ability(Ability::static_ability(StaticAbility::protection(
                ProtectionFrom::ChosenColor,
            )))
            .build();
        let donor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.set_chosen_color(donor, Color::Red);
        let recipient = counter_creature(&mut game, Zone::Battlefield);
        if let Some(color) = prior_choice {
            game.set_chosen_color(recipient, color);
        }
        let mut ctx = EffectContext::new_default(recipient, alice);
        ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(recipient),
            RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(donor),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: vec![],
                copy_exception_surface: None,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            game.chosen_color(recipient),
            prior_choice,
            "CR 707.6: donor choice must not replace recipient's own choice"
        );
        CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(donor))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let token = game
            .battlefield
            .iter()
            .copied()
            .find(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .unwrap();
        assert_eq!(
            game.chosen_color(token),
            None,
            "New copy has no inherited chosen color"
        );
        for (color, source_colors) in [(Color::Red, ColorSet::RED), (Color::Blue, ColorSet::BLUE)] {
            let source_def = CardDefinitionBuilder::new(CardId::new(), "Colored damage source")
                .card_types(vec![CardType::Artifact])
                .color_indicator(source_colors)
                .build();
            let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
            for (target, chosen) in [
                (donor, Some(Color::Red)),
                (recipient, prior_choice),
                (token, None),
            ] {
                let before = game.damage_on(target);
                DealDamageEffect::new(1, ChooseSpec::SpecificObject(target))
                    .execute(&mut game, &mut EffectContext::new_default(source, alice))
                    .unwrap();
                assert_eq!(
                    game.damage_on(target) - before,
                    u32::from(chosen != Some(color)),
                    "Copied protection reads the receiving object's choice: color={color:?}, prior={prior_choice:?}"
                );
            }
        }
    }
}

#[test]
fn cr_707_6_copy_entry_makes_its_own_as_enters_color_choice() {
    use ironsmith::ability::ProtectionFrom;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::ColorsContext;
    use ironsmith::effects::{CreateTokenCopyEffect, DealDamageEffect, MoveToZoneEffect};
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Color, ColorSet, StaticAbility, Zone};
    struct EntryColors {
        players: Vec<PlayerId>,
    }
    impl DecisionMaker for EntryColors {
        fn decide_colors(&mut self, _game: &GameState, ctx: &ColorsContext) -> Vec<Color> {
            self.players.push(ctx.player);
            vec![if self.players.len() == 1 {
                Color::Red
            } else {
                Color::Blue
            }]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Fresh entry choice")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 10))
        .with_ability(Ability::static_ability(
            StaticAbility::choose_color_as_enters(None, "As this enters, choose a color.".into()),
        ))
        .with_ability(Ability::static_ability(StaticAbility::protection(
            ProtectionFrom::ChosenColor,
        )))
        .build();
    let card = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    let mut dm = EntryColors {
        players: Vec::new(),
    };
    MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
        .execute(
            &mut game,
            &mut EffectContext::new_default(card, alice).with_decision_maker(&mut dm),
        )
        .unwrap();
    let donor = game.battlefield[0];
    assert_eq!(game.chosen_color(donor), Some(Color::Red));
    CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(donor))
        .execute(
            &mut game,
            &mut EffectContext::new_default(donor, bob).with_decision_maker(&mut dm),
        )
        .unwrap();
    let copy = game
        .battlefield
        .iter()
        .copied()
        .find(|id| *id != donor)
        .unwrap();
    assert_eq!(
        dm.players,
        vec![alice, bob],
        "CR 707.6: each entrant's controller makes a separate choice"
    );
    assert_eq!(game.chosen_color(donor), Some(Color::Red));
    assert_eq!(game.chosen_color(copy), Some(Color::Blue));
    assert_eq!(game.current_controller(copy), Some(bob));
    for (color, colors) in [(Color::Red, ColorSet::RED), (Color::Blue, ColorSet::BLUE)] {
        let source_def = CardDefinitionBuilder::new(CardId::new(), "Damage source")
            .card_types(vec![CardType::Artifact])
            .color_indicator(colors)
            .build();
        let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        for (target, chosen) in [(donor, Color::Red), (copy, Color::Blue)] {
            let before = game.damage_on(target);
            DealDamageEffect::new(1, ChooseSpec::SpecificObject(target))
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            assert_eq!(game.damage_on(target) - before, u32::from(color != chosen));
        }
    }
}

#[test]
fn cr_708_9_face_down_zone_departures_reveal_identity_to_all_players() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, Zone};
    let bob = PlayerId::from_index(1);
    for origin in [Zone::Battlefield, Zone::Stack] {
        for destination in [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
            Zone::Battlefield,
        ] {
            if origin == destination {
                continue;
            }
            for face_down in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let definition = CardDefinitionBuilder::new(CardId::new(), "Revealed identity")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                    .build();
                let object = game.create_object_from_definition(&definition, bob, origin);
                let stable = game.object(object).unwrap().stable_id;
                if face_down {
                    assert!(game.set_face_down(object));
                }
                let before = game.ui_effect_events().map(|event| event.id).max();
                let moved = game.move_object_by_effect(object, destination).unwrap();
                let reveals: Vec<_> = game
                    .ui_effect_events()
                    .filter(|event| {
                        before.is_none_or(|id| event.id > id)
                            && event.kind == "reveal"
                            && event.stable_ids.contains(&stable)
                    })
                    .collect();
                let should_reveal =
                    face_down && !(origin == Zone::Stack && destination == Zone::Battlefield);
                assert_eq!(
                    reveals.len(),
                    usize::from(should_reveal),
                    "CR 708.9: {origin:?}->{destination:?}, face down={face_down}"
                );
                if should_reveal {
                    assert_eq!(reveals[0].player, Some(bob));
                    assert_eq!(reveals[0].text.as_deref(), Some("Revealed identity"));
                }
                if origin == Zone::Stack && destination == Zone::Battlefield && face_down {
                    assert!(
                        game.is_face_down(moved),
                        "Resolving face-down spell stays face down without revealing"
                    );
                }
            }
        }
    }
}

#[test]
fn cr_406_3a_and_708_9_face_down_exile_has_no_characteristics_after_required_reveal() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::ExileEffect;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, ColorSet, StaticAbility, Zone};
    let bob = PlayerId::from_index(1);
    for origin in [Zone::Battlefield, Zone::Stack] {
        for initially_face_down in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Hidden again")
                .card_types(vec![CardType::Creature])
                .color_indicator(ColorSet::GREEN)
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .with_ability(Ability::static_ability(StaticAbility::flying()))
                .build();
            let object = game.create_object_from_definition(&definition, bob, origin);
            let stable = game.object(object).unwrap().stable_id;
            if initially_face_down {
                assert!(game.set_face_down(object));
            }
            let before = game.ui_effect_events().map(|event| event.id).max();
            ExileEffect::with_spec(ChooseSpec::SpecificObject(object))
                .with_face_down(true)
                .execute(&mut game, &mut EffectContext::new_default(object, bob))
                .unwrap();
            let exiled = game.exile[0];
            assert!(game.is_face_down(exiled));
            let reveals: Vec<_> = game
                .ui_effect_events()
                .filter(|event| {
                    before.is_none_or(|id| event.id > id)
                        && event.kind == "reveal"
                        && event.stable_ids.contains(&stable)
                })
                .collect();
            assert_eq!(
                reveals.len(),
                usize::from(initially_face_down),
                "CR 708.9: explicit face-down exile does not suppress the old face-down object's reveal"
            );
            if initially_face_down {
                assert_eq!(reveals[0].text.as_deref(), Some("Hidden again"));
            }
            let chars = game.current_characteristics(exiled).unwrap();
            assert!(chars.name.is_empty());
            assert!(
                chars.card_types.is_empty()
                    && chars.subtypes.is_empty()
                    && chars.supertypes.is_empty()
            );
            assert!(
                chars.colors.is_empty() && chars.mana_cost.is_none() && chars.abilities.is_empty()
            );
            assert_eq!((chars.power, chars.toughness), (None, None));
            let returned = game.move_object_by_effect(exiled, Zone::Hand).unwrap();
            assert!(!game.is_face_down(returned));
            assert_eq!(game.current_name(returned).as_deref(), Some("Hidden again"));
            assert_eq!(
                (
                    game.current_power(returned),
                    game.current_toughness(returned)
                ),
                (Some(3), Some(4))
            );
            assert!(game.object_has_ability(returned, &StaticAbility::flying()));
        }
    }
}

#[test]
fn cr_406_3_exile_inspection_permission_survives_source_control_and_zone_changes() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, ExileEffect};
    use ironsmith::{ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let source = counter_creature(&mut game, Zone::Battlefield);
    let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::CardId::new(),
        "Secret exiled card",
    )
    .card_types(vec![ironsmith::CardType::Creature])
    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
    .build();
    let card = game.create_object_from_definition(&definition, carol, Zone::Library);
    ExileEffect::with_spec(ChooseSpec::SpecificObject(card))
        .with_face_down(true)
        .with_source_controller_look()
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    let exiled = game.exile[0];
    assert!(game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert!(!game.can_player_look_at_face_down_exiled_card(exiled, bob));
    assert!(
        !game.can_player_look_at_face_down_exiled_card(exiled, carol),
        "Ownership alone grants no face-down exile inspection permission"
    );
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::ChangeController(bob),
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, alice))
    .unwrap();
    game.refresh_continuous_state().unwrap();
    for viewer in [alice, bob] {
        assert!(game.can_player_look_at_face_down_exiled_card(exiled, viewer));
    }
    game.phase_out(source);
    for viewer in [alice, bob] {
        assert!(
            game.can_player_look_at_face_down_exiled_card(exiled, viewer),
            "Entitled player need not have actually looked and keeps permission after source phases out"
        );
    }
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    for viewer in [alice, bob] {
        assert!(game.can_player_look_at_face_down_exiled_card(exiled, viewer));
    }
    let reexiled = game.move_object_by_effect(exiled, Zone::Exile).unwrap();
    assert_ne!(reexiled, exiled);
    for viewer in [alice, bob] {
        assert!(
            game.can_player_look_at_face_down_exiled_card(reexiled, viewer),
            "Exiling again within exile does not leave the exile zone"
        );
    }
    let hand = game.move_object_by_effect(reexiled, Zone::Hand).unwrap();
    let fresh_exile = game.move_object_by_effect(hand, Zone::Exile).unwrap();
    game.set_face_down(fresh_exile);
    for viewer in [alice, bob, carol] {
        assert!(
            !game.can_player_look_at_face_down_exiled_card(fresh_exile, viewer),
            "Leaving exile ends the old inspection entitlement"
        );
    }
}

#[test]
fn cr_104_4a_simultaneous_life_losses_draw_only_when_no_players_remain() {
    use ironsmith::{GameProgress, GameResult};
    for count in 2..=4 {
        for losers in 1..(1u32 << count) {
            for life in [0, -1] {
                let mut game =
                    GameState::new((0..count).map(|i| format!("Player {i}")).collect(), 20);
                let survivors: Vec<_> = (0..count)
                    .filter(|i| losers & (1 << i) == 0)
                    .map(PlayerId::from_index)
                    .collect();
                for i in 0..count {
                    if losers & (1 << i) != 0 {
                        game.player_mut(PlayerId::from_index(i)).unwrap().life = life;
                    }
                }
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let progress =
                    ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
                let remaining: Vec<_> = game
                    .players
                    .iter()
                    .filter(|player| !player.has_lost && !player.has_left_game)
                    .map(|player| player.id)
                    .collect();
                assert_eq!(
                    remaining, survivors,
                    "All zero/negative-life losses occur at same priority boundary"
                );
                match survivors.as_slice() {
                    [] => assert!(
                        matches!(progress, GameProgress::GameOver(GameResult::Draw)),
                        "CR 104.4a: count={count}, losers={losers}, result={progress:?}"
                    ),
                    [winner] => assert!(
                        matches!(progress,GameProgress::GameOver(GameResult::Winner(id)) if id == *winner),
                        "CR 104.2a: sole survivor wins"
                    ),
                    _ => assert!(
                        !matches!(progress, GameProgress::GameOver(_)),
                        "Two or more remaining FFA players continue"
                    ),
                }
            }
        }
    }
}

#[test]
fn cr_104_3_cannot_lose_suppresses_lethal_states_and_old_failed_draw_expires() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DrawCardsEffect;
    use ironsmith::{Ability, CardId, CardType, GameProgress, GameResult, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for cause in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Cannot lose source")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(StaticAbility::you_cant_lose_game()))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.player_mut(bob).unwrap().life = 0;
        match cause {
            0 => game.player_mut(alice).unwrap().life = 0,
            1 => game.player_mut(alice).unwrap().poison_counters = 10,
            _ => {
                DrawCardsEffect::you(1)
                    .execute(&mut game, &mut EffectContext::new_default(source, alice))
                    .unwrap();
            }
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let progress = ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
        assert!(!matches!(progress, GameProgress::GameOver(_)));
        assert!(!game.player(alice).unwrap().has_lost);
        assert!(game.player(bob).unwrap().has_lost);
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        let progress = ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
        if cause < 2 {
            assert!(
                matches!(progress,GameProgress::GameOver(GameResult::Winner(id)) if id == carol)
            );
            assert!(game.player(alice).unwrap().has_lost);
        } else {
            assert!(
                !matches!(progress, GameProgress::GameOver(_)),
                "CR 704.5b: a failed draw checked under cannot-lose does not remain a pending loss forever"
            );
            assert!(!game.player(alice).unwrap().has_lost);
            DrawCardsEffect::you(1)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            let progress = ironsmith::game_loop::advance_priority(&mut game, &mut queue).unwrap();
            assert!(
                matches!(progress,GameProgress::GameOver(GameResult::Winner(id)) if id == carol)
            );
            assert!(game.player(alice).unwrap().has_lost);
        }
    }
}

#[test]
fn cr_104_3a_concession_ignores_cannot_lose_and_removes_owned_objects_immediately() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, Effect, StaticAbility, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for cannot_lose in [false, true] {
        for has_priority in [false, true] {
            for pending_ability in [false, true] {
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                game.turn.priority_player = Some(if has_priority { alice } else { bob });
                let mut builder = CardDefinitionBuilder::new(CardId::new(), "Concession source")
                    .card_types(vec![CardType::Enchantment]);
                if cannot_lose {
                    builder = builder
                        .with_ability(Ability::static_ability(StaticAbility::you_cant_lose_game()));
                }
                let source =
                    game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
                let mut owned = vec![source];
                for zone in [Zone::Hand, Zone::Library, Zone::Graveyard, Zone::Exile] {
                    owned.push(counter_creature(&mut game, zone));
                }
                let other_def = CardDefinitionBuilder::new(CardId::new(), "Borrowed creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                    .build();
                let borrowed =
                    game.create_object_from_definition(&other_def, bob, Zone::Battlefield);
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(borrowed),
                    Modification::ChangeController(alice),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
                assert_eq!(game.current_controller(borrowed), Some(alice));
                if pending_ability {
                    game.stack.push(ironsmith::StackEntry::ability(
                        source,
                        alice,
                        vec![Effect::gain_life(1)],
                    ));
                }
                assert!(
                    game.concede_game(alice).unwrap(),
                    "CR 104.3a: concession is unconditional"
                );
                assert!(game.player(alice).unwrap().has_lost);
                assert!(game.player(alice).unwrap().has_left_game);
                for object in owned {
                    assert!(
                        game.object(object).is_none(),
                        "CR 800.4a: owned objects leave immediately"
                    );
                }
                assert_eq!(
                    game.current_controller(borrowed),
                    Some(bob),
                    "Control effects giving the departed player control end"
                );
                assert!(
                    game.stack.is_empty(),
                    "Departed player's ability stack objects cease to exist"
                );
                assert_eq!(game.turn.priority_player, Some(bob));
                game.validate_zone_consistency().unwrap();
            }
        }
    }
}

#[test]
fn cr_800_4a_c_departed_default_controller_exiles_remaining_permanents() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, BattlefieldController, MoveToZoneEffect};
    use ironsmith::{CardId, CardType, ChooseSpec, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for owner in [bob, carol] {
        for temporary_control in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Foreign-owned entrant")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
            let card = game.create_object_from_definition(&definition, owner, Zone::Graveyard);
            let stable = game.object(card).unwrap().stable_id;
            let mut enter =
                MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false);
            enter.battlefield_controller = BattlefieldController::You;
            enter
                .execute(&mut game, &mut EffectContext::new_default(card, alice))
                .unwrap();
            let permanent = game.battlefield[0];
            assert_eq!(game.current_controller(permanent), Some(alice));
            assert_eq!(game.object(permanent).unwrap().owner, owner);
            if temporary_control {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(permanent),
                    Modification::ChangeController(bob),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(permanent, bob))
                .unwrap();
                assert_eq!(game.current_controller(permanent), Some(bob));
            }
            game.concede_game(alice).unwrap();
            if temporary_control {
                assert!(game.battlefield.contains(&permanent));
                assert_eq!(game.current_controller(permanent), Some(bob));
                ironsmith::turn::execute_cleanup_step(&mut game);
                game.refresh_continuous_state().unwrap();
            }
            assert!(
                !game.battlefield.contains(&permanent),
                "CR 800.4a/c: permanent cannot remain controlled by departed default controller"
            );
            let exiled = game
                .exile
                .iter()
                .copied()
                .find(|id| game.object(*id).unwrap().stable_id == stable)
                .expect("Foreign-owned permanent is exiled rather than removed from game");
            assert_eq!(game.object(exiled).unwrap().owner, owner);
            assert_eq!(
                game.current_name(exiled).as_deref(),
                Some("Foreign-owned entrant")
            );
            assert_ne!(permanent, exiled);
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn cr_800_4b_departed_players_cannot_gain_control_receive_permanents_or_create_tokens() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{
        ApplyContinuousEffect, BattlefieldController, CreateTokenEffect, MoveToZoneEffect,
    };
    use ironsmith::{CardId, CardType, ChooseSpec, PlayerFilter, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Recipient test creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .build();
        let permanent = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let card = game.create_object_from_definition(&definition, carol, Zone::Graveyard);
        if departed {
            game.concede_game(alice).unwrap();
        }
        ApplyContinuousEffect::new(
            EffectTarget::Specific(permanent),
            Modification::ChangeController(alice),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(permanent, bob))
        .unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            game.current_controller(permanent),
            Some(if departed { bob } else { alice }),
            "CR 800.4b: departed player cannot gain control"
        );
        let mut token_effect = CreateTokenEffect::one(definition);
        token_effect.controller = PlayerFilter::Specific(alice);
        token_effect
            .execute(&mut game, &mut EffectContext::new_default(permanent, bob))
            .unwrap();
        let token_count = game
            .battlefield
            .iter()
            .filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .count();
        assert_eq!(
            token_count,
            usize::from(!departed),
            "CR 800.4b/d: no token for departed player"
        );
        let mut move_effect =
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false);
        move_effect.battlefield_controller = BattlefieldController::You;
        move_effect
            .execute(&mut game, &mut EffectContext::new_default(permanent, alice))
            .unwrap();
        if departed {
            assert_eq!(
                game.object(card).unwrap().zone,
                Zone::Graveyard,
                "Entry under departed controller leaves card in original zone"
            );
        } else {
            assert!(game.object(card).is_none());
            assert_eq!(game.battlefield.len(), 3);
        }
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn cr_800_4d_departed_players_pending_triggers_are_not_put_on_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DestroyEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Effect, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for departure_case in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let departing_def = CardDefinitionBuilder::new(CardId::new(), "Departing source")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::triggered(
                Trigger::this_leaves_battlefield(),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let departing =
            game.create_object_from_definition(&departing_def, alice, Zone::Battlefield);
        let observer_def = CardDefinitionBuilder::new(CardId::new(), "Surviving observer")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::triggered(
                Trigger::leaves_battlefield(ObjectFilter::creature()),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let observer = game.create_object_from_definition(&observer_def, bob, Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        if departure_case == 0 {
            DestroyEffect::with_spec(ChooseSpec::SpecificObject(departing))
                .execute(&mut game, &mut EffectContext::new_default(observer, bob))
                .unwrap();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert_eq!(
                queue.entries.len(),
                2,
                "Both players' abilities trigger before departure"
            );
        }
        if departure_case == 2 {
            game.phase_out(departing);
        }
        game.concede_game(alice).unwrap();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.stack.len(),
            usize::from(departure_case != 2),
            "Phased-out permanent departure does not trigger LTB; other departures retain survivor trigger"
        );
        if departure_case != 2 {
            assert_eq!(game.stack[0].object_id, observer);
            assert_eq!(game.stack[0].controller, bob);
        }
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn cr_800_4e_no_combat_damage_is_assigned_to_departed_defending_player() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{
        Ability, CardId, CardType, Phase, StaticAbility, Step, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Lifelink attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(Ability::static_ability(StaticAbility::haste()))
            .with_ability(Ability::static_ability(StaticAbility::lifelink()))
            .build();
        let attacker = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut combat = combat_state::new_combat();
        combat_state::declare_attackers(
            &mut game,
            &mut combat,
            vec![(attacker, AttackTarget::Player(bob))],
        )
        .unwrap();
        game.turn.step = Some(Step::DeclareBlockers);
        combat_state::declare_blockers(&game, &mut combat, vec![]).unwrap();
        game.combat = Some(combat);
        if departed {
            game.concede_game(bob).unwrap();
        }
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
        *runner.combat_mut() = game.combat.clone().unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..2 {
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::Continue
            ));
        }
        assert_eq!(
            game.player(bob).unwrap().life,
            if departed { 20 } else { 17 },
            "CR 800.4e: departed defender receives no assigned damage"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if departed { 20 } else { 23 },
            "No damage means no lifelink gain"
        );
        assert_eq!(game.player(carol).unwrap().life, 20);
        assert!(game.battlefield.contains(&attacker));
    }
}

#[test]
fn cr_800_4g_nonpayment_choice_must_not_be_requested_from_departed_player() {
    use ironsmith::effects::ChooseColorEffect;
    use ironsmith::{Color, PlayerFilter, Zone};
    struct InspectChoosers {
        players: Vec<PlayerId>,
    }
    impl DecisionMaker for InspectChoosers {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.players.push(ctx.player);
            vec![0]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Choice source",
        )
        .card_types(vec![ironsmith::CardType::Artifact])
        .build();
        let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        if departed {
            game.concede_game(alice).unwrap();
        }
        let mut dm = InspectChoosers {
            players: Vec::new(),
        };
        ChooseColorEffect::new(PlayerFilter::Specific(alice))
            .execute(
                &mut game,
                &mut EffectContext::new_default(source, bob).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(game.chosen_color(source), Some(Color::White));
        assert!(!dm.players.is_empty());
        if departed {
            results.push(
                dm.players
                    .iter()
                    .all(|player| game.player(*player).unwrap().is_in_game()),
            );
        } else {
            assert_eq!(dm.players, vec![alice], "Live designated chooser control");
        }
    }
    assert_eq!(
        results,
        vec![true],
        "CR 800.4g: resolving object's controller must choose another player; a departed player cannot answer a nonpayment choice"
    );
}

#[test]
fn cr_800_4f_departed_player_never_chooses_or_pays_an_unless_cost() {
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::{Effect, PlayerFilter, Value, Zone};
    struct AcceptPayments {
        asked: Vec<PlayerId>,
    }
    impl DecisionMaker for AcceptPayments {
        fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
            self.asked.push(ctx.player);
            true
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut departed_results = Vec::new();
    for departed in [false, true] {
        for life_cost in [0, 3] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Unless source",
            )
            .card_types(vec![ironsmith::CardType::Artifact])
            .build();
            let source = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            if departed {
                game.concede_game(alice).unwrap();
            }
            let mut dm = AcceptPayments { asked: Vec::new() };
            UnlessPaysEffect::new_with_life(
                vec![Effect::gain_life(1)],
                PlayerFilter::Specific(alice),
                vec![],
                Some(Value::Fixed(life_cost)),
            )
            .execute(
                &mut game,
                &mut EffectContext::new_default(source, bob).with_decision_maker(&mut dm),
            )
            .unwrap();
            if departed {
                departed_results.push((
                    dm.asked.len(),
                    game.player(alice).unwrap().life,
                    game.player(bob).unwrap().life,
                ));
            } else {
                assert_eq!(dm.asked, vec![alice]);
                assert_eq!(game.player(alice).unwrap().life, 20 - life_cost);
                assert_eq!(
                    game.player(bob).unwrap().life,
                    20,
                    "Accepted live payment prevents the consequence"
                );
            }
        }
    }
    assert_eq!(
        departed_results,
        vec![(0, 20, 21), (0, 20, 21)],
        "CR 800.4f: departed player is not asked to pay, cannot pay even a zero cost, and the unpaid consequence occurs"
    );
}

#[test]
fn cr_800_4i_departed_player_values_use_last_known_hand_and_life_information() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::GainLifeEffect;
    use ironsmith::{CardId, CardType, PlayerFilter, Value, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        game.player_mut(bob).unwrap().life = 13;
        let card = CardDefinitionBuilder::new(CardId::new(), "Last known hand card")
            .card_types(vec![CardType::Artifact])
            .build();
        for _ in 0..3 {
            game.create_object_from_definition(&card, bob, Zone::Hand);
        }
        let hand: Vec<_> = game.player(bob).unwrap().hand.iter().copied().collect();
        if departed {
            game.concede_game(bob).unwrap();
            for id in hand {
                assert!(game.object(id).is_none());
            }
        }
        for (value, expected) in [
            (Value::LifeTotal(PlayerFilter::Specific(bob)), 13),
            (Value::CardsInHand(PlayerFilter::Specific(bob)), 3),
            (
                Value::HalfLifeTotalRoundedUp(PlayerFilter::Specific(bob)),
                7,
            ),
            (
                Value::HalfLifeTotalRoundedDown(PlayerFilter::Specific(bob)),
                6,
            ),
        ] {
            let before = game.player(alice).unwrap().life;
            GainLifeEffect::you(value)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            assert_eq!(
                game.player(alice).unwrap().life - before,
                expected,
                "CR 800.4i: specific-player information survives departure; departed={departed}"
            );
        }
    }
}

#[test]
fn cr_800_4m_departed_players_next_turn_duration_lasts_through_preceding_cleanup() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{CardId, CardType, Phase, Step, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut cleanup_values = Vec::new();
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let target = counter_creature(&mut game, Zone::Battlefield);
        let source_def = CardDefinitionBuilder::new(CardId::new(), "Duration source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&source_def, bob, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2,
            },
            Until::YourNextTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, bob))
        .unwrap();
        if departed {
            game.concede_game(bob).unwrap();
        }
        assert_eq!(game.turn.active_player, alice);
        assert_eq!(game.turn.phase, Phase::FirstMain);
        assert_eq!(
            (game.current_power(target), game.current_toughness(target)),
            (Some(5), Some(6)),
            "Departure does not expire the effect immediately"
        );
        game.turn.phase = Phase::Ending;
        game.turn.step = Some(Step::Cleanup);
        ironsmith::turn::execute_cleanup_step(&mut game);
        cleanup_values.push((game.current_power(target), game.current_toughness(target)));
        game.next_turn();
        assert_eq!(
            game.turn.active_player,
            if departed { carol } else { bob },
            "CR 800.4k: departed player's next turn does not begin"
        );
        assert_eq!(
            (game.current_power(target), game.current_toughness(target)),
            (Some(3), Some(4)),
            "Duration has expired by the next surviving player's turn"
        );
    }
    assert_eq!(
        cleanup_values,
        vec![(Some(5), Some(6)), (Some(5), Some(6))],
        "CR 800.4m: until next turn does not become until end of the preceding turn when that player leaves"
    );
}

#[test]
fn cr_800_4j_end_step_still_triggers_after_active_player_leaves() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, Phase, PlayerFilter, Step, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut trigger_counts = Vec::new();
    for departed in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        game.turn.active_player = alice;
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let turn_number = game.turn.turn_number;
        let definition = CardDefinitionBuilder::new(CardId::new(), "Every end-step observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::beginning_of_end_step(PlayerFilter::Any),
                vec![Effect::gain_life(1)],
            ))
            .build();
        let observer = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        if departed {
            game.concede_game(alice).unwrap();
            assert_eq!(game.active_player_id(), None);
        }
        assert_eq!(game.turn.phase, Phase::FirstMain);
        assert_eq!(game.turn.turn_number, turn_number);
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(game.turn.phase, Phase::Ending);
        assert_eq!(game.turn.step, Some(Step::End));
        assert_eq!(game.turn.turn_number, turn_number);
        assert_eq!(
            game.turn.priority_player,
            Some(if departed { bob } else { alice })
        );
        trigger_counts.push(
            queue
                .entries
                .iter()
                .filter(|entry| entry.source == observer)
                .count(),
        );
    }
    assert_eq!(
        trigger_counts,
        vec![1, 1],
        "CR 800.4j: current turn continues without active player, including beginning-of-end-step abilities"
    );
}

#[test]
fn cr_800_4j_cleanup_finishes_departed_players_turn_without_discard_requests() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect};
    use ironsmith::{
        CardId, CardType, ChooseSpec, Phase, Step, TurnAction, TurnRunner, TurnRunnerState, Until,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::Cleanup);
    let turn_number = game.turn.turn_number;
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Cleanup survivor")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    let survivor = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    for owner in [alice, bob] {
        for _ in 0..9 {
            game.create_object_from_definition(&creature_def, owner, Zone::Hand);
        }
    }
    let mut ctx = EffectContext::new_default(survivor, bob);
    ApplyContinuousEffect::new(
        EffectTarget::Specific(survivor),
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2,
        },
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(survivor))
        .execute(&mut game, &mut ctx)
        .unwrap();
    game.concede_game(alice).unwrap();
    assert_eq!(game.active_player_id(), None);
    assert!(ironsmith::turn::get_cleanup_discard_spec(&game).is_none());
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CleanupDiscard);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for _ in 0..3 {
        assert!(
            matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::Continue
            ),
            "Cleanup with no active player has no discard choice or ordinary priority"
        );
    }
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::TurnComplete
    ));
    assert_eq!(game.turn.turn_number, turn_number);
    assert_eq!(
        game.player(bob).unwrap().hand.len(),
        9,
        "Nonactive survivor is not forced to discard during departed player's cleanup"
    );
    assert_eq!(game.damage_on(survivor), 0);
    assert_eq!(
        (
            game.current_power(survivor),
            game.current_toughness(survivor)
        ),
        (Some(3), Some(4))
    );
    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
}

#[test]
fn cr_800_4n_ante_cards_survive_owner_departure_and_transfer_to_winner() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, GameProgress, GameResult, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let mut anted = Vec::new();
    for owner in [alice, bob, carol] {
        for index in 0..4 {
            let definition = CardDefinitionBuilder::new(
                CardId::new(),
                format!("Ante candidate {owner:?} {index}"),
            )
            .card_types(vec![CardType::Artifact])
            .build();
            game.create_object_from_definition(&definition, owner, Zone::Library);
        }
        let ante = game.ante_random_library_card(owner).unwrap();
        assert_eq!(game.object(ante).unwrap().owner, owner);
        assert_eq!(game.object(ante).unwrap().zone, Zone::Ante);
        assert_eq!(game.player(owner).unwrap().library.len(), 3);
        anted.push((ante, owner, game.object(ante).unwrap().stable_id));
    }
    let foreign = game.player(alice).unwrap().library[0];
    assert!(
        game.ante_owned_object(bob, foreign).is_err(),
        "CR 407.4: only owner may ante an object"
    );
    assert_eq!(game.object(foreign).unwrap().zone, Zone::Library);
    game.concede_game(alice).unwrap();
    for (id, owner, stable) in &anted {
        assert_eq!(
            game.object(*id).unwrap().owner,
            *owner,
            "Ante ownership is preserved until game ends"
        );
        assert_eq!(game.object(*id).unwrap().stable_id, *stable);
        assert!(game.ante.contains(id));
    }
    game.concede_game(carol).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    assert!(
        matches!(ironsmith::game_loop::advance_priority(&mut game,&mut queue).unwrap(),GameProgress::GameOver(GameResult::Winner(id)) if id==bob)
    );
    for (id, _, stable) in anted {
        assert_eq!(
            game.object(id).unwrap().owner,
            bob,
            "CR 407.2: winner becomes owner of every ante card, including departed owners' cards"
        );
        assert_eq!(game.object(id).unwrap().stable_id, stable);
        assert_eq!(game.object(id).unwrap().zone, Zone::Ante);
    }
    game.validate_zone_consistency().unwrap();
}

#[test]
fn cr_704_5q_opposing_counters_cancel_by_minimum_and_wait_while_phased_out() {
    use ironsmith::{CounterType, Zone};
    for plus in 0u32..6 {
        for minus in 0u32..6 {
            for phased in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Counter balance",
                )
                .card_types(vec![ironsmith::CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 20))
                .build();
                let object = game.create_object_from_definition(
                    &definition,
                    PlayerId::from_index(0),
                    Zone::Battlefield,
                );
                game.add_counters(object, CounterType::PlusOnePlusOne, plus);
                game.add_counters(object, CounterType::MinusOneMinusOne, minus);
                game.add_counters(object, CounterType::Charge, 2);
                if phased {
                    game.phase_out(object);
                }
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                if phased {
                    assert_eq!(
                        game.counter_count(object, CounterType::PlusOnePlusOne),
                        plus
                    );
                    assert_eq!(
                        game.counter_count(object, CounterType::MinusOneMinusOne),
                        minus
                    );
                    game.phase_in(object);
                    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                }
                assert_eq!(
                    game.counter_count(object, CounterType::PlusOnePlusOne),
                    plus.saturating_sub(minus)
                );
                assert_eq!(
                    game.counter_count(object, CounterType::MinusOneMinusOne),
                    minus.saturating_sub(plus)
                );
                assert_eq!(game.counter_count(object, CounterType::Charge), 2);
            }
        }
    }
}

#[test]
fn cr_704_5r_counter_limits_remove_only_excess_while_ability_applies() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CounterType, StaticAbility, Until, Zone};
    let alice = PlayerId::from_index(0);
    for maximum in 0u32..4 {
        for count in 0u32..6 {
            for mode in 0..3 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Limited counters",
                )
                .card_types(vec![ironsmith::CardType::Artifact])
                .with_ability(Ability::static_ability(StaticAbility::counter_limit_rule(
                    CounterType::Dream,
                    maximum,
                    "Counter limit",
                )))
                .with_ability(Ability::static_ability(StaticAbility::counter_limit_rule(
                    CounterType::Dream,
                    maximum + 2,
                    "Larger counter limit",
                )))
                .build();
                let object =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                game.add_counters(object, CounterType::Dream, count);
                game.add_counters(object, CounterType::Charge, 2);
                assert_eq!(
                    game.counter_count(object, CounterType::Dream),
                    count,
                    "Counter limit is applied by state-based action"
                );
                if mode == 1 {
                    game.phase_out(object);
                }
                if mode == 2 {
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(object),
                        Modification::RemoveAllAbilities,
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(object, alice))
                    .unwrap();
                }
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                assert_eq!(
                    game.counter_count(object, CounterType::Dream),
                    if mode == 0 { count.min(maximum) } else { count }
                );
                if mode == 1 {
                    game.phase_in(object);
                }
                if mode == 2 {
                    ironsmith::turn::execute_cleanup_step(&mut game);
                }
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                assert_eq!(
                    game.counter_count(object, CounterType::Dream),
                    count.min(maximum)
                );
                assert_eq!(game.counter_count(object, CounterType::Charge), 2);
            }
        }
    }
}

#[test]
fn cr_704_8_death_trigger_snapshot_precedes_simultaneous_counter_annihilation() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, CounterType, Effect, Zone};
    let alice = PlayerId::from_index(0);
    for persist in [false, true] {
        for plus in 0u32..3 {
            for minus in 0u32..4 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let builder =
                    CardDefinitionBuilder::new(CardId::new(), "Simultaneous SBA snapshot")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
                        .with_ability(Ability::triggered(
                            Trigger::this_dies(),
                            vec![Effect::gain_life(1)],
                        ));
                let definition = (if persist {
                    builder.persist()
                } else {
                    builder.undying()
                })
                .build();
                let creature =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                game.add_counters(creature, CounterType::PlusOnePlusOne, plus);
                game.add_counters(creature, CounterType::MinusOneMinusOne, minus);
                DealDamageEffect::new(10, ChooseSpec::SpecificObject(creature))
                    .execute(&mut game, &mut EffectContext::new_default(creature, alice))
                    .unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                assert!(!game.battlefield.contains(&creature));
                assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
                let keyword_should_trigger = if persist { minus == 0 } else { plus == 0 };
                assert_eq!(
                    queue.entries.len(),
                    1 + usize::from(keyword_should_trigger),
                    "CR 704.8: persist={persist}, plus={plus}, minus={minus}"
                );
                for entry in &queue.entries {
                    let snapshot = entry.source_snapshot.as_ref().unwrap();
                    assert_eq!(
                        snapshot
                            .counters
                            .get(&CounterType::PlusOnePlusOne)
                            .copied()
                            .unwrap_or(0),
                        plus
                    );
                    assert_eq!(
                        snapshot
                            .counters
                            .get(&CounterType::MinusOneMinusOne)
                            .copied()
                            .unwrap_or(0),
                        minus
                    );
                }
            }
        }
    }
}

#[test]
fn cr_603_6c_persist_and_undying_return_only_the_first_graveyard_incarnation() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DestroyEffect};
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut returned_cases = Vec::new();
    for persist in [false, true] {
        for path in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let builder = CardDefinitionBuilder::new(CardId::new(), "Returning mechanic")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4));
            let definition = (if persist {
                builder.persist()
            } else {
                builder.undying()
            })
            .build();
            let creature = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let stable = game.object(creature).unwrap().stable_id;
            ApplyContinuousEffect::new(
                EffectTarget::Specific(creature),
                Modification::ChangeController(alice),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(creature, alice))
            .unwrap();
            DestroyEffect::with_spec(ChooseSpec::SpecificObject(creature))
                .execute(&mut game, &mut EffectContext::new_default(creature, alice))
                .unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert_eq!(queue.entries.len(), 1);
            assert_eq!(queue.entries[0].controller, alice);
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            let grave = game.player(bob).unwrap().graveyard[0];
            if path > 0 {
                let exiled = game.move_object_by_effect(grave, Zone::Exile).unwrap();
                if path == 2 {
                    game.move_object_by_effect(exiled, Zone::Graveyard).unwrap();
                }
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let returned = game
                .battlefield
                .iter()
                .copied()
                .find(|id| game.object(*id).unwrap().stable_id == stable);
            returned_cases.push(returned.is_some());
            if let Some(object) = returned {
                assert_eq!(
                    game.current_controller(object),
                    Some(bob),
                    "Return is under owner control, regardless of trigger controller"
                );
                assert_eq!(
                    game.counter_count(
                        object,
                        if persist {
                            CounterType::MinusOneMinusOne
                        } else {
                            CounterType::PlusOnePlusOne
                        }
                    ),
                    1
                );
                assert_eq!(
                    (game.current_power(object), game.current_toughness(object)),
                    if persist {
                        (Some(2), Some(3))
                    } else {
                        (Some(4), Some(5))
                    }
                );
            }
        }
    }
    assert_eq!(
        returned_cases,
        vec![true, false, false, true, false, false],
        "CR 603.6c/400.7e: a death trigger cannot follow a card out of its first graveyard incarnation and back"
    );
}

#[test]
fn cr_603_4_intervening_if_checks_trigger_time_and_resolution_for_ability_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Condition;
    use ironsmith::effects::{BattlefieldController, MoveToZoneEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for trigger_life in [9, 10] {
        for resolution_life in [9, 10] {
            for source_leaves in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.player_mut(bob).unwrap().life = trigger_life;
                let mut ability = Ability::triggered(
                    Trigger::this_enters_battlefield(),
                    vec![Effect::gain_life(3)],
                );
                let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
                    unreachable!()
                };
                triggered.intervening_if = Some(Condition::LifeTotalOrGreater(10));
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Intervening condition source")
                        .card_types(vec![CardType::Artifact])
                        .with_ability(ability)
                        .build();
                let card = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
                let mut enter = MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(card),
                    Zone::Battlefield,
                    false,
                );
                enter.battlefield_controller = BattlefieldController::You;
                enter
                    .execute(&mut game, &mut EffectContext::new_default(card, bob))
                    .unwrap();
                let source = game.battlefield[0];
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue)
                    .unwrap();
                assert_eq!(queue.entries.len(), usize::from(trigger_life >= 10));
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                game.player_mut(bob).unwrap().life = resolution_life;
                if source_leaves {
                    game.move_object_by_effect(source, Zone::Exile).unwrap();
                }
                if trigger_life >= 10 {
                    assert_eq!(game.stack[0].controller, bob);
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                assert!(game.stack.is_empty());
                assert_eq!(
                    game.player(bob).unwrap().life,
                    resolution_life
                        + if trigger_life >= 10 && resolution_life >= 10 {
                            3
                        } else {
                            0
                        },
                    "CR 603.4: trigger life={trigger_life}, resolution life={resolution_life}, source leaves={source_leaves}"
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20,
                    "Condition and effect use controller rather than owner"
                );
            }
        }
    }
}

#[test]
fn cr_603_3b_simultaneous_triggers_use_controller_choice_and_apnap_stack_order() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::OrderContext;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, ObjectId, TurnAction, TurnRunner, TurnRunnerState, Zone,
    };
    struct OrderTriggers {
        reverse: bool,
        seen: Vec<PlayerId>,
    }
    impl DecisionMaker for OrderTriggers {
        fn decide_order(&mut self, _: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
            assert_eq!(ctx.items.len(), 2);
            self.seen.push(ctx.player);
            let mut items = ctx.items.clone();
            items.sort_by(|a, b| a.1.cmp(&b.1));
            if self.reverse {
                items.reverse();
            }
            items.into_iter().map(|(id, _)| id).collect()
        }
    }
    for players in 2..=4 {
        for active in 0..players {
            for reverse in [false, true] {
                let mut game =
                    GameState::new((0..players).map(|p| format!("Player {p}")).collect(), 20);
                game.turn.active_player = PlayerId::from_index(active);
                let mut sources = Vec::new();
                for player in 0..players {
                    let owner = PlayerId::from_index(player);
                    let mut pair = Vec::new();
                    for name in ["A ordered trigger", "Z ordered trigger"] {
                        let definition = CardDefinitionBuilder::new(CardId::new(), name)
                            .card_types(vec![CardType::Enchantment])
                            .with_ability(Ability::triggered(
                                Trigger::beginning_of_end_step(PlayerFilter::Any),
                                vec![Effect::gain_life(1)],
                            ))
                            .build();
                        pair.push(game.create_object_from_definition(
                            &definition,
                            owner,
                            Zone::Battlefield,
                        ));
                    }
                    sources.push(pair);
                }
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                assert_eq!(queue.entries.len(), usize::from(players) * 2);
                let mut dm = OrderTriggers {
                    reverse,
                    seen: Vec::new(),
                };
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                    .unwrap();
                let apnap: Vec<_> = (0..players)
                    .map(|offset| (active + offset) % players)
                    .collect();
                assert_eq!(
                    dm.seen,
                    apnap
                        .iter()
                        .map(|p| PlayerId::from_index(*p))
                        .collect::<Vec<_>>()
                );
                let mut expected = Vec::new();
                for player in apnap {
                    // The order decision explicitly requests top-first order. Stack storage is bottom-first.
                    let top_first = if reverse { [1, 0] } else { [0, 1] };
                    for index in top_first.into_iter().rev() {
                        expected.push((
                            sources[usize::from(player)][index],
                            PlayerId::from_index(player),
                        ));
                    }
                }
                assert_eq!(
                    game.stack
                        .iter()
                        .map(|entry| (entry.object_id, entry.controller))
                        .collect::<Vec<_>>(),
                    expected,
                    "CR 603.3b: players={players}, active={active}, reverse={reverse}"
                );
                while !game.stack.is_empty() {
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
                for player in 0..players {
                    assert_eq!(game.player(PlayerId::from_index(player)).unwrap().life, 22);
                }
            }
        }
    }
}

#[test]
fn cr_603_3b_trigger_on_trigger_abilities_use_a_second_complete_apnap_pass() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, ObjectFilter, TurnAction, TurnRunner, TurnRunnerState,
        Zone,
    };
    for players in 2..=4 {
        for active in 0..players {
            let mut game =
                GameState::new((0..players).map(|p| format!("Player {p}")).collect(), 20);
            game.turn.active_player = PlayerId::from_index(active);
            let mut ordinary_sources = Vec::new();
            let mut observer_sources = Vec::new();
            for player in 0..players {
                let owner = PlayerId::from_index(player);
                let ordinary =
                    CardDefinitionBuilder::new(CardId::new(), "Ordinary end-step trigger")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
                        .with_ability(Ability::triggered(
                            Trigger::beginning_of_end_step(PlayerFilter::Any),
                            vec![Effect::gain_life(1)],
                        ))
                        .build();
                ordinary_sources.push(game.create_object_from_definition(
                    &ordinary,
                    owner,
                    Zone::Battlefield,
                ));
                // Observes creature sources only, so these enchantment abilities cannot trigger each other.
                let observer = CardDefinitionBuilder::new(CardId::new(), "Trigger observer")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::triggered(
                        Trigger::ability_triggered_qualified(
                            false,
                            Some(ObjectFilter::creature()),
                            false,
                        ),
                        vec![Effect::gain_life(2)],
                    ))
                    .build();
                observer_sources.push(game.create_object_from_definition(
                    &observer,
                    owner,
                    Zone::Battlefield,
                ));
            }
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            let mut expected = Vec::new();
            for offset in 0..players {
                let player = (active + offset) % players;
                expected.push((
                    ordinary_sources[usize::from(player)],
                    PlayerId::from_index(player),
                ));
            }
            for offset in 0..players {
                let player = (active + offset) % players;
                for _ in 0..players {
                    expected.push((
                        observer_sources[usize::from(player)],
                        PlayerId::from_index(player),
                    ));
                }
            }
            assert_eq!(
                game.stack
                    .iter()
                    .map(|entry| (entry.object_id, entry.controller))
                    .collect::<Vec<_>>(),
                expected,
                "CR 603.3b: both APNAP passes must finish in sequence, players={players}, active={active}"
            );
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            for player in 0..players {
                assert_eq!(
                    game.player(PlayerId::from_index(player)).unwrap().life,
                    21 + 2 * players as i32
                );
            }
        }
    }
}

#[test]
fn cr_603_3a_trigger_controller_is_fixed_when_it_triggers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, TurnAction, TurnRunner, TurnRunnerState, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for before_stacking in [false, true] {
        for leaves in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Controller at trigger time")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::triggered(
                        Trigger::beginning_of_end_step(PlayerFilter::Any),
                        vec![Effect::gain_life(3)],
                    ))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::ChangeController(bob),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(queue.entries.len(), 1);
            assert_eq!(queue.entries[0].controller, bob);
            if !before_stacking {
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            }
            if leaves {
                game.move_object_by_effect(source, Zone::Exile).unwrap();
            } else {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(source),
                    Modification::ChangeController(alice),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
                assert_eq!(game.current_controller(source), Some(alice));
            }
            if before_stacking {
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            }
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.stack[0].controller, bob);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                (
                    game.player(alice).unwrap().life,
                    game.player(bob).unwrap().life
                ),
                (20, 23),
                "CR 603.3a: controller captured at trigger time survives source change; before stacking={before_stacking}, leaves={leaves}"
            );
        }
    }
}

#[test]
fn cr_603_5_optional_and_unless_choices_happen_only_when_trigger_resolves() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MayEffect, UnlessPaysEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, TurnAction, TurnRunner, TurnRunnerState, Value, Zone,
    };
    struct AnswerOptional {
        accept: bool,
        asked: Vec<PlayerId>,
    }
    impl DecisionMaker for AnswerOptional {
        fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
            self.asked.push(ctx.player);
            self.accept
        }
    }
    let alice = PlayerId::from_index(0);
    for unless in [false, true] {
        for accept in [false, true] {
            for source_leaves in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let optional = if unless {
                    Effect::new(UnlessPaysEffect::new_with_life(
                        vec![Effect::gain_life(3)],
                        PlayerFilter::You,
                        vec![],
                        Some(Value::Fixed(3)),
                    ))
                } else {
                    Effect::new(MayEffect::single(Effect::gain_life(3)))
                };
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Optional end-step ability")
                        .card_types(vec![CardType::Enchantment])
                        .with_ability(Ability::triggered(
                            Trigger::beginning_of_end_step(PlayerFilter::Any),
                            vec![optional],
                        ))
                        .build();
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                assert_eq!(
                    queue.entries.len(),
                    1,
                    "The optional effect does not prevent triggering"
                );
                let mut dm = AnswerOptional {
                    accept,
                    asked: Vec::new(),
                };
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                    .unwrap();
                assert_eq!(game.stack.len(), 1);
                assert!(
                    dm.asked.is_empty(),
                    "CR 603.5: no may/unless decision during stack placement"
                );
                assert_eq!(game.player(alice).unwrap().life, 20);
                if source_leaves {
                    game.move_object_by_effect(source, Zone::Exile).unwrap();
                }
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.asked, vec![alice]);
                assert!(game.stack.is_empty());
                let expected = if unless {
                    if accept { 17 } else { 23 }
                } else if accept {
                    23
                } else {
                    20
                };
                assert_eq!(
                    game.player(alice).unwrap().life,
                    expected,
                    "CR 603.5: unless={unless}, accept={accept}, source leaves={source_leaves}"
                );
                assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
            }
        }
    }
}

#[test]
fn cr_603_2e_entry_state_and_repeated_tap_instructions_do_not_trigger_transitions() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Effect, Zone};
    let alice = PlayerId::from_index(0);
    for enters_tapped in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Tap-state observer")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(4)],
            ))
            .with_ability(Ability::triggered(
                Trigger::becomes_tapped(),
                vec![Effect::gain_life(1)],
            ))
            .with_ability(Ability::triggered(
                Trigger::becomes_untapped(),
                vec![Effect::gain_life(2)],
            ))
            .build();
        let card = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let mut enter =
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false);
        enter.enters_tapped = enters_tapped;
        enter
            .execute(&mut game, &mut EffectContext::new_default(card, alice))
            .unwrap();
        let source = game.battlefield[0];
        assert_eq!(game.is_tapped(source), enters_tapped);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert_eq!(
            queue.entries.len(),
            1,
            "CR 603.2e: entry state cannot trigger becomes tapped/untapped"
        );
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 24);
        let mut life = 24;
        for tap in [true, true, false, false, true, true, false] {
            let changed = game.is_tapped(source) != tap;
            let effect = if tap {
                Effect::tap(ChooseSpec::SpecificObject(source))
            } else {
                Effect::untap(ChooseSpec::SpecificObject(source))
            };
            game.stack
                .push(ironsmith::StackEntry::ability(source, alice, vec![effect]));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.is_tapped(source), tap);
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert_eq!(
                queue.entries.len(),
                usize::from(changed),
                "CR 603.2e: only actual tap-state transitions trigger"
            );
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            if changed {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                life += if tap { 1 } else { 2 };
            }
            assert_eq!(game.player(alice).unwrap().life, life);
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_603_3d_required_trigger_targets_are_chosen_at_stack_placement() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, TurnAction, TurnRunner, TurnRunnerState,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    // 0: never had a legal target; 1: target leaves before placement;
    // 2: target leaves after placement; 3: target stays legal.
    for path in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target_spec = ChooseSpec::target_creature();
        let mut ability = Ability::triggered(
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![
                Effect::new(DealDamageEffect::new(1, target_spec.clone())),
                Effect::gain_life(3),
            ],
        );
        let ironsmith::AbilityKind::Triggered(ref mut triggered) = ability.kind else {
            unreachable!()
        };
        triggered.choices = vec![target_spec];
        let definition = CardDefinitionBuilder::new(CardId::new(), "Targeted end-step ability")
            .card_types(vec![CardType::Enchantment])
            .with_ability(ability)
            .build();
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let target = if path == 0 {
            None
        } else {
            Some(counter_creature(&mut game, Zone::Battlefield))
        };
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(
            queue.entries.len(),
            1,
            "Target availability does not prevent the ability from triggering"
        );
        if path == 1 {
            game.move_object_by_effect(target.unwrap(), Zone::Exile)
                .unwrap();
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.stack.len(),
            usize::from(path >= 2),
            "CR 603.3d: no mandatory target means ability removed; path={path}"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        if path == 2 {
            game.move_object_by_effect(target.unwrap(), Zone::Exile)
                .unwrap();
        }
        if path >= 2 {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        assert_eq!(
            game.player(alice).unwrap().life,
            if path == 3 { 23 } else { 20 },
            "CR 608.2b: all targets illegal prevents the entire ability, including nontargeted life gain; path={path}"
        );
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_603_3c_modal_trigger_announces_only_legal_modes_and_their_targets() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::effect::EffectMode;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, ObjectFilter, Target, TurnAction,
        TurnRunner, TurnRunnerState, Zone,
    };
    struct ChooseModal {
        preferred: usize,
        available: [bool; 2],
        chosen: Option<usize>,
        prompts: Vec<&'static str>,
    }
    impl DecisionMaker for ChooseModal {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.prompts.push("mode");
            assert_eq!(
                ctx.options
                    .iter()
                    .map(|o| (o.index, o.legal))
                    .collect::<Vec<_>>(),
                vec![(0, self.available[0]), (1, self.available[1])]
            );
            let chosen = if self.available[self.preferred] {
                self.preferred
            } else {
                1 - self.preferred
            };
            assert!(self.available[chosen]);
            self.chosen = Some(chosen);
            vec![chosen]
        }
        fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.prompts.push("target");
            assert_eq!(self.prompts, vec!["mode", "target"]);
            assert_eq!(
                ctx.requirements.len(),
                1,
                "Only selected mode has a target requirement"
            );
            assert_eq!(ctx.requirements[0].legal_targets.len(), 1);
            vec![ctx.requirements[0].legal_targets[0]]
        }
    }
    let alice = PlayerId::from_index(0);
    for creature_available in [false, true] {
        for artifact_available in [false, true] {
            for preferred in 0..2 {
                for source_leaves in [false, true] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    let modes = vec![
                        EffectMode::new(
                            "Exile target creature",
                            vec![Effect::exile(ChooseSpec::target_creature())],
                        ),
                        EffectMode::new(
                            "Exile target artifact",
                            vec![Effect::exile(ChooseSpec::target(ChooseSpec::Object(
                                ObjectFilter::artifact(),
                            )))],
                        ),
                    ];
                    let definition =
                        CardDefinitionBuilder::new(CardId::new(), "Modal end-step source")
                            .card_types(vec![CardType::Enchantment])
                            .with_ability(Ability::triggered(
                                Trigger::beginning_of_end_step(PlayerFilter::Any),
                                vec![Effect::choose_one(modes)],
                            ))
                            .build();
                    let source =
                        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    let creature =
                        creature_available.then(|| counter_creature(&mut game, Zone::Battlefield));
                    let artifact = artifact_available.then(|| {
                        let def =
                            CardDefinitionBuilder::new(CardId::new(), "Modal artifact subject")
                                .card_types(vec![CardType::Artifact])
                                .build();
                        game.create_object_from_definition(&def, alice, Zone::Battlefield)
                    });
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
                    assert!(matches!(
                        runner.advance(&mut game, &mut queue).unwrap(),
                        TurnAction::RunPriority
                    ));
                    assert_eq!(queue.entries.len(), 1);
                    let mut dm = ChooseModal {
                        preferred,
                        available: [creature_available, artifact_available],
                        chosen: None,
                        prompts: vec![],
                    };
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(
                        &mut game, &mut queue, &mut dm,
                    )
                    .unwrap();
                    if !creature_available && !artifact_available {
                        assert!(
                            game.stack.is_empty(),
                            "CR 603.3c: no legal mode removes the triggered ability"
                        );
                        assert!(dm.prompts.is_empty());
                        continue;
                    }
                    assert_eq!(game.stack.len(), 1);
                    assert_eq!(dm.prompts, vec!["mode", "target"]);
                    let chosen = dm.chosen.unwrap();
                    assert_eq!(game.stack[0].chosen_modes.as_deref(), Some(&[chosen][..]));
                    let targets = [creature, artifact];
                    assert_eq!(
                        game.stack[0].targets,
                        vec![Target::Object(targets[chosen].unwrap())]
                    );
                    if source_leaves {
                        game.move_object_by_effect(source, Zone::Exile).unwrap();
                    }
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(
                        dm.prompts,
                        vec!["mode", "target"],
                        "Mode and target choices do not repeat on resolution"
                    );
                    for (mode, target) in targets.into_iter().enumerate() {
                        if let Some(target) = target {
                            assert_eq!(
                                game.object(target).is_some(),
                                mode != chosen,
                                "Only the chosen mode exiles its target"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn cr_603_3c_choosing_zero_optional_modes_removes_trigger_without_effects() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::EffectMode;
    use ironsmith::effects::ChooseModeEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, TurnAction, TurnRunner, TurnRunnerState, Value, Zone,
    };
    struct ChooseZero {
        asked: usize,
    }
    impl DecisionMaker for ChooseZero {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            self.asked += 1;
            assert_eq!((ctx.min, ctx.max), (0, 1));
            vec![]
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let modes = ChooseModeEffect::new(
        vec![EffectMode::new(
            "Gain three life",
            vec![Effect::gain_life(3)],
        )],
        Value::Fixed(0),
        Value::Fixed(1),
        false,
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), "Optional modal trigger")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            Trigger::beginning_of_end_step(PlayerFilter::Any),
            vec![Effect::new(modes), Effect::gain_life(7)],
        ))
        .build();
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(queue.entries.len(), 1);
    let mut dm = ChooseZero { asked: 0 };
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(dm.asked, 1);
    assert!(
        game.stack.is_empty(),
        "CR 603.3c: choosing zero modes removes the entire ability, including subsequent effects"
    );
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn cr_603_7a_delayed_trigger_cannot_observe_an_event_before_registration() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MoveToZoneEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    // Registration is before entry, after entry with its event pending, or after that event was drained.
    for path in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source_def = CardDefinitionBuilder::new(CardId::new(), "Delayed-registration source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let subject = counter_creature(&mut game, Zone::Graveyard);
        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::enters_battlefield(ObjectFilter::creature(), None),
            vec![Effect::gain_life(3)],
            true,
            vec![],
            PlayerFilter::You,
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        if path == 0 {
            schedule
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
        }
        MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(subject),
            Zone::Battlefield,
            false,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        if path == 2 {
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(queue.entries.is_empty());
        }
        if path != 0 {
            schedule
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
        }
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        let triggered = queue.entries.len();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        results.push((triggered, game.player(alice).unwrap().life));
    }
    assert_eq!(
        results,
        vec![(1, 23), (0, 20), (0, 20)],
        "CR 603.7a: event buffering cannot make a delayed ability trigger retroactively; only the registration preceding entry sees it"
    );
}

#[test]
fn cr_603_7a_linking_a_delayed_trigger_to_a_tag_does_not_replay_prior_entry() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MoveToZoneEffect, ScheduleDelayedTriggerEffect, TaggedEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, Zone};
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    for drain_before_registration in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Linked delayed-registration source")
                .card_types(vec![CardType::Artifact])
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let subject = counter_creature(&mut game, Zone::Graveyard);
        let mut ctx = EffectContext::new_default(source, alice);
        TaggedEffect::new(
            "returned",
            Effect::new(MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(subject),
                Zone::Battlefield,
                false,
            )),
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(ctx.get_tagged_all("returned").unwrap().len(), 1);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        if drain_before_registration {
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(queue.entries.is_empty());
        }
        ScheduleDelayedTriggerEffect::from_tag(
            Trigger::this_enters_battlefield(),
            vec![Effect::gain_life(3)],
            true,
            "returned",
            PlayerFilter::You,
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        drop(ctx);
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        let count = queue.entries.len();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        results.push((count, game.player(alice).unwrap().life));
    }
    assert_eq!(
        results,
        vec![(0, 20), (0, 20)],
        "CR 603.7a: delayed registration after a tagged return cannot observe that earlier return, even while its event is pending"
    );
}

#[test]
fn cr_603_7c_delayed_effect_keeps_exact_incarnation_despite_characteristic_changes() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        CardId, CardType, ChooseSpec, Effect, TurnAction, TurnRunner, TurnRunnerState, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    for before_stacking in [false, true] {
        for path in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Delayed exact-object source")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let subject = counter_creature(&mut game, Zone::Battlefield);
            let stable = game.object(subject).unwrap().stable_id;
            let mut ctx = EffectContext::new_default(source, alice);
            ctx.tag_object(
                "watched",
                ObjectSnapshot::from_object(game.object(subject).unwrap(), &game),
            );
            ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(PlayerFilter::Any),
                vec![Effect::exile(ChooseSpec::Tagged("watched".into()))],
                true,
                vec![],
                PlayerFilter::You,
            )
            .execute(&mut game, &mut ctx)
            .unwrap();
            drop(ctx);
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            if !before_stacking {
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                assert_eq!(game.stack.len(), 1);
            }
            let current_subject = match path {
                1 => {
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(subject),
                        Modification::SetCardTypes(vec![CardType::Artifact]),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(source, alice))
                    .unwrap();
                    subject
                }
                2 | 3 => {
                    let exiled = game.move_object_by_effect(subject, Zone::Exile).unwrap();
                    if path == 3 {
                        game.move_object_by_effect(exiled, Zone::Battlefield)
                            .unwrap()
                    } else {
                        exiled
                    }
                }
                _ => subject,
            };
            if before_stacking {
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                assert_eq!(game.stack.len(), 1);
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let final_id = game
                .battlefield
                .iter()
                .chain(game.exile.iter())
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable)
                })
                .unwrap();
            results.push((
                game.object(final_id).unwrap().zone,
                final_id != current_subject,
            ));
        }
    }
    let expected = vec![
        (Zone::Exile, true),
        (Zone::Exile, true),
        (Zone::Exile, false),
        (Zone::Battlefield, false),
    ];
    assert_eq!(
        results,
        [expected.clone(), expected].concat(),
        "CR 603.7c: delayed reference survives type change but never re-exiles or follows another incarnation, before or after stacking"
    );
}

#[test]
fn cr_603_7d_spell_creates_delayed_trigger_with_its_resolution_controller_and_source() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::ScheduleDelayedTriggerEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        CardId, CardType, Effect, StackEntry, TurnAction, TurnRunner, TurnRunnerState, Zone,
    };
    let bob = PlayerId::from_index(1);
    for card_moves_again in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Spell creating delayed ability")
                .card_types(vec![CardType::Sorcery])
                .with_spell_effect(vec![Effect::new(ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(PlayerFilter::Any),
                    vec![Effect::gain_life(3)],
                    true,
                    vec![],
                    PlayerFilter::You,
                ))])
                .build();
        let spell = game.create_object_from_definition(&definition, bob, Zone::Stack);
        let stable = game.object(spell).unwrap().stable_id;
        game.stack.push(StackEntry::new(spell, bob));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert!(
            game.object(spell).is_none(),
            "Resolving spell becomes a new graveyard object"
        );
        assert_eq!(game.player(bob).unwrap().graveyard.len(), 1);
        if card_moves_again {
            let card = game.player(bob).unwrap().graveyard[0];
            game.move_object_by_effect(card, Zone::Exile).unwrap();
        }
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.entries[0].controller, bob);
        assert_eq!(
            queue.entries[0].source, spell,
            "CR 603.7d: source is the original spell, not its later card incarnation"
        );
        assert_eq!(
            queue.entries[0].source_snapshot.as_ref().unwrap().stable_id,
            stable
        );
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(
            (game.stack[0].object_id, game.stack[0].controller),
            (spell, bob)
        );
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (index, expected) in [(0, 20), (1, 23), (2, 20)] {
            assert_eq!(
                game.player(PlayerId::from_index(index)).unwrap().life,
                expected
            );
        }
    }
}

#[test]
fn cr_603_7e_delayed_trigger_uses_creating_ability_controller_not_current_source_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, Effect, StackEntry, TurnAction, TurnRunner, TurnRunnerState,
        Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for triggered_creator in [false, true] {
        for source_leaves in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let schedule = Effect::new(ScheduleDelayedTriggerEffect::new(
                Trigger::beginning_of_end_step(PlayerFilter::Any),
                vec![Effect::gain_life(3)],
                true,
                vec![],
                PlayerFilter::You,
            ));
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Ability creating delayed ability")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::triggered(
                        Trigger::beginning_of_upkeep(PlayerFilter::Any),
                        vec![schedule.clone()],
                    ))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::ChangeController(bob),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            if triggered_creator {
                let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Upkeep);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            } else {
                game.stack
                    .push(StackEntry::ability(source, bob, vec![schedule]));
            }
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.stack[0].controller, bob);
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::ChangeController(carol),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            assert_eq!(game.current_controller(source), Some(carol));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            if source_leaves {
                game.move_object_by_effect(source, Zone::Exile).unwrap();
            } else {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(source),
                    Modification::ChangeController(alice),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            }
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(queue.entries.len(), 1);
            assert_eq!(
                (queue.entries[0].source, queue.entries[0].controller),
                (source, bob)
            );
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(
                (game.stack[0].object_id, game.stack[0].controller),
                (source, bob)
            );
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                (
                    game.player(alice).unwrap().life,
                    game.player(bob).unwrap().life,
                    game.player(carol).unwrap().life
                ),
                (20, 23, 20),
                "CR 603.7e: creating ability's controller Bob controls delayed ability despite owner Alice/current controller Carol and later source loss"
            );
        }
    }
}

#[test]
fn cr_603_7b_one_shot_and_this_turn_delayed_registrations_have_distinct_lifetimes() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MoveToZoneEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    for this_turn in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Delayed lifetime source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let schedule = ScheduleDelayedTriggerEffect::new(
            Trigger::enters_battlefield(ObjectFilter::creature(), None),
            vec![Effect::gain_life(1)],
            !this_turn,
            vec![],
            PlayerFilter::You,
        );
        let schedule = if this_turn {
            schedule.until_end_of_turn()
        } else {
            schedule
        };
        schedule
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let mut counts = Vec::new();
        for event in 0..3 {
            if event == 2 {
                ironsmith::turn::execute_cleanup_step(&mut game);
                game.next_turn();
            }
            let subject = counter_creature(&mut game, Zone::Graveyard);
            MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(subject),
                Zone::Battlefield,
                false,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            counts.push(queue.entries.len());
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
        }
        assert_eq!(
            counts,
            if this_turn {
                vec![1, 1, 0]
            } else {
                vec![1, 0, 0]
            },
            "CR 603.7b: a one-shot fires only on first event; this-turn watcher fires on each event before expiration"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            if this_turn { 22 } else { 21 }
        );
    }
}

#[test]
fn cr_603_7b_controller_selects_the_event_for_simultaneous_one_shot_occurrences() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MoveToZoneEffect, ScheduleDelayedTriggerEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ObjectFilter, Zone};
    struct SelectEvent {
        name: &'static str,
        asked: usize,
    }
    impl DecisionMaker for SelectEvent {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(
                ctx.player,
                PlayerId::from_index(1),
                "Delayed controller, rather than active player, chooses event"
            );
            assert_eq!((ctx.min, ctx.max, ctx.options.len()), (1, 1, 2));
            self.asked += 1;
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.description.contains(self.name))
                    .unwrap()
                    .index,
            ]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for chosen_name in ["A simultaneous entrant", "Z simultaneous entrant"] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source_def =
            CardDefinitionBuilder::new(CardId::new(), "One-shot simultaneous entry observer")
                .card_types(vec![CardType::Artifact])
                .build();
        let source = game.create_object_from_definition(&source_def, bob, Zone::Battlefield);
        let mut subjects = Vec::new();
        for (name, owner) in [
            ("A simultaneous entrant", alice),
            ("Z simultaneous entrant", bob),
        ] {
            let def = CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
            let id = game.create_object_from_definition(&def, owner, Zone::Graveyard);
            subjects.push((name, game.object(id).unwrap().stable_id));
        }
        ScheduleDelayedTriggerEffect::new(
            Trigger::enters_battlefield(ObjectFilter::creature(), None),
            vec![Effect::exile(ChooseSpec::Tagged("triggering".into()))],
            true,
            vec![],
            PlayerFilter::You,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, bob))
        .unwrap();
        MoveToZoneEffect::new(
            ChooseSpec::All(ObjectFilter::creature().in_zone(Zone::Graveyard)),
            Zone::Battlefield,
            false,
        )
        .under_owner_control()
        .execute(&mut game, &mut EffectContext::new_default(source, bob))
        .unwrap();
        assert_eq!(game.battlefield.len(), 3);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert_eq!(
            queue.entries.len(),
            1,
            "One-shot produces only one triggered ability for the simultaneous events"
        );
        let mut dm = SelectEvent {
            name: chosen_name,
            asked: 0,
        };
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        assert_eq!(
            dm.asked, 1,
            "CR 603.7b: controller chooses which simultaneous occurrence causes one-shot trigger"
        );
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].controller, bob);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.asked, 1);
        for (name, stable) in subjects {
            let id = game
                .battlefield
                .iter()
                .chain(game.exile.iter())
                .copied()
                .find(|id| game.object(*id).is_some_and(|o| o.stable_id == stable))
                .unwrap();
            assert_eq!(
                game.object(id).unwrap().zone,
                if name == chosen_name {
                    Zone::Exile
                } else {
                    Zone::Battlefield
                },
                "Only selected occurrence's creature is exiled"
            );
        }
        let later = counter_creature(&mut game, Zone::Graveyard);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(later), Zone::Battlefield, false)
            .execute(&mut game, &mut EffectContext::new_default(source, bob))
            .unwrap();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert!(
            queue.entries.is_empty(),
            "Consumed one-shot cannot trigger for a later event"
        );
    }
}

#[test]
fn cr_603_8_state_trigger_condition_does_not_become_a_resolution_condition() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Condition;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, ObjectFilter, Subtype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut sacrificed = Vec::new();
    for gain_swamp_after_trigger in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::triggered(
            Trigger::state_based("When you control no Swamps"),
            vec![Effect::sacrifice_source()],
        );
        let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
            unreachable!()
        };
        // Mirrors the compiler's StateBased lowering, which currently stores the trigger condition here.
        triggered.intervening_if = Some(Condition::Not(Box::new(Condition::YouControl(
            ObjectFilter::land().with_subtype(Subtype::Swamp),
        ))));
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "No-Swamps state-trigger source")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        if gain_swamp_after_trigger {
            let swamp_def = CardDefinitionBuilder::new(CardId::new(), "Swamp gained in response")
                .card_types(vec![CardType::Land])
                .subtypes(vec![Subtype::Swamp])
                .build();
            let swamp = game.create_object_from_definition(&swamp_def, alice, Zone::Graveyard);
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(swamp), Zone::Battlefield, false)
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        sacrificed.push(game.player(alice).unwrap().graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable)
        }));
    }
    assert_eq!(
        sacrificed,
        vec![true, true],
        "CR 603.8/603.4: a state-trigger condition is not an intervening if; gaining a Swamp after triggering cannot stop the pending sacrifice"
    );
}

#[test]
fn cr_603_8_new_source_incarnation_can_trigger_while_old_incarnation_ability_is_pending() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Condition;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, Effect, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut ability = Ability::triggered(
        Trigger::state_based("When you have ten or more life"),
        vec![Effect::gain_life(1)],
    );
    let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
        unreachable!()
    };
    triggered.intervening_if = Some(Condition::LifeTotalOrGreater(10));
    let definition = CardDefinitionBuilder::new(CardId::new(), "Reentering state-trigger source")
        .card_types(vec![CardType::Artifact])
        .with_ability(ability)
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let stable = game.object(source).unwrap().stable_id;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert_eq!(queue.entries.len(), 1);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    let exile = game.move_object_by_effect(source, Zone::Exile).unwrap();
    let returned = game
        .move_object_by_effect(exile, Zone::Battlefield)
        .unwrap();
    assert_ne!(returned, source);
    assert_eq!(game.object(returned).unwrap().stable_id, stable);
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "The old object's ability remains pending"
    );
    assert_eq!(
        queue.entries.len(),
        1,
        "CR 603.8/400.7: the new object's ability is independent and must trigger even while the old object's state trigger is on the stack"
    );
    assert_eq!(queue.entries[0].source, returned);
}

#[test]
fn cr_603_8_state_trigger_rearms_after_resolution_or_countering_without_condition_edge() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Condition;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    for counter in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::triggered(
            Trigger::state_based("When you have ten or more life"),
            vec![Effect::gain_life(1)],
        );
        let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
            unreachable!()
        };
        triggered.intervening_if = Some(Condition::LifeTotalOrGreater(10));
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "State-trigger lifecycle source")
                .card_types(vec![CardType::Artifact])
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for _ in 0..3 {
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert_eq!(
                queue.entries.len(),
                1,
                "Pending unstacked state trigger suppresses duplicates"
            );
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        for life in [20, 9, 20] {
            game.player_mut(alice).unwrap().life = life;
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                queue.entries.is_empty(),
                "Neither repeated checks nor false/true state cycling may retrigger while pending"
            );
            assert_eq!(game.stack.len(), 1);
        }
        if counter {
            let counter_def =
                CardDefinitionBuilder::new(CardId::new(), "Countering ability source")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let counter_source =
                game.create_object_from_definition(&counter_def, alice, Zone::Battlefield);
            game.stack.push(StackEntry::ability(
                counter_source,
                alice,
                vec![Effect::counter(ChooseSpec::SpecificObject(source))],
            ));
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert!(game.object(source).is_some());
        assert_eq!(
            game.player(alice).unwrap().life,
            if counter { 20 } else { 21 }
        );
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(
            queue.entries.len(),
            1,
            "CR 603.8: resolution or countering rearms a still-true state trigger without any false-to-true edge"
        );
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.len(), 1);
    }
}

#[test]
fn cr_603_8_transient_state_during_resolution_still_triggers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Condition;
    use ironsmith::effects::{MoveToZoneEffect, TaggedEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, ObjectFilter, StackEntry,
        Subtype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    for return_swamp_in_same_resolution in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::triggered(
            Trigger::state_based("When you control no Swamps"),
            vec![Effect::sacrifice_source()],
        );
        let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
            unreachable!()
        };
        triggered.intervening_if = Some(Condition::Not(Box::new(Condition::YouControl(
            ObjectFilter::land().with_subtype(Subtype::Swamp),
        ))));
        let source_def = CardDefinitionBuilder::new(CardId::new(), "Transient no-Swamps observer")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
            .with_ability(ability)
            .build();
        let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
        let swamp_def = CardDefinitionBuilder::new(CardId::new(), "Temporarily absent Swamp")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Swamp])
            .build();
        let swamp = game.create_object_from_definition(&swamp_def, alice, Zone::Battlefield);
        let swamp_stable = game.object(swamp).unwrap().stable_id;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert!(
            queue.entries.is_empty(),
            "Initial state condition is false because a Swamp is controlled"
        );
        let resolution_source_def =
            CardDefinitionBuilder::new(CardId::new(), "Transient-state instruction source")
                .card_types(vec![CardType::Artifact])
                .build();
        let resolution_source =
            game.create_object_from_definition(&resolution_source_def, alice, Zone::Battlefield);
        let mut effects = vec![Effect::new(TaggedEffect::new(
            "temporary_swamp",
            Effect::exile(ChooseSpec::SpecificObject(swamp)),
        ))];
        if return_swamp_in_same_resolution {
            effects.push(Effect::new(
                MoveToZoneEffect::new(
                    ChooseSpec::Tagged("temporary_swamp".into()),
                    Zone::Battlefield,
                    false,
                )
                .under_owner_control(),
            ));
        }
        game.stack
            .push(StackEntry::ability(resolution_source, alice, effects));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let swamp_present = game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|o| o.stable_id == swamp_stable)
        });
        assert_eq!(
            swamp_present, return_swamp_in_same_resolution,
            "Fixture must restore the Swamp before the resolution ends"
        );
        assert!(
            game.object(source).is_some(),
            "State trigger waits for stack placement rather than resolving within the other ability"
        );
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        results.push(queue.entries.len());
    }
    assert_eq!(
        results,
        vec![1, 1],
        "CR 603.8: state trigger occurs as soon as Swamp is absent; returning it later in the same resolution cannot erase the trigger"
    );
}

#[test]
fn cr_603_8_state_trigger_occurs_before_sbas_remove_the_qualifying_creature() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effect::Condition;
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::filter::Comparison;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ObjectFilter, StackEntry, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut counts = Vec::new();
    for toughness_reduction in [2, 4] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut filter = ObjectFilter::creature();
        filter.toughness = Some(Comparison::LessThanOrEqual(2));
        let mut ability = Ability::triggered(
            Trigger::state_based("When you control a creature with toughness 2 or less"),
            vec![Effect::sacrifice_source()],
        );
        let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
            unreachable!()
        };
        triggered.intervening_if = Some(Condition::YouControl(filter));
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Small-toughness state observer")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(4, 5))
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let subject = counter_creature(&mut game, Zone::Battlefield);
        let subject_stable = game.object(subject).unwrap().stable_id;
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert!(queue.entries.is_empty());
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![Effect::new(ApplyContinuousEffect::new(
                EffectTarget::Specific(subject),
                Modification::ModifyPowerToughness {
                    power: 0,
                    toughness: -toughness_reduction,
                },
                Until::EndOfTurn,
            ))],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.current_toughness(subject),
            Some(4 - toughness_reduction)
        );
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(
            game.player(alice).unwrap().graveyard.iter().any(|id| game
                .object(*id)
                .is_some_and(|o| o.stable_id == subject_stable)),
            toughness_reduction == 4
        );
        assert!(game.object(source).is_some());
        counts.push(queue.entries.len());
    }
    assert_eq!(
        counts,
        vec![1, 1],
        "CR 603.8: toughness 0 satisfies the state trigger before the creature dies to an SBA, just as toughness 2 does"
    );
}

#[test]
fn cr_603_9_and_603_10f_player_loss_uses_surviving_controllers_predeparture_observer() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::{ApplyContinuousEffect, DrawCardsEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, Effect, StackEntry, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for owner in [alice, bob] {
        for cause in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Predeparture loss observer")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::triggered(
                        Trigger::player_loses_game(PlayerFilter::Specific(alice)),
                        vec![Effect::gain_life(3)],
                    ))
                    .build();
            let observer =
                game.create_object_from_definition(&definition, owner, Zone::Battlefield);
            if owner == alice {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(observer),
                    Modification::ChangeController(bob),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(observer, bob))
                .unwrap();
            }
            assert_eq!(game.current_controller(observer), Some(bob));
            match cause {
                0 => {
                    game.concede_game(alice).unwrap();
                }
                1 => game.player_mut(alice).unwrap().life = 0,
                2 => game.player_mut(alice).unwrap().poison_counters = 10,
                _ => {
                    assert!(game.player(alice).unwrap().library.is_empty());
                    game.stack.push(StackEntry::ability(
                        observer,
                        bob,
                        vec![Effect::new(DrawCardsEffect::new(
                            1,
                            PlayerFilter::Specific(alice),
                        ))],
                    ));
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
            }
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                game.player(alice).unwrap().has_lost && game.player(alice).unwrap().has_left_game
            );
            assert_eq!(
                game.object(observer).is_none(),
                owner == alice,
                "Departing player's owned observer leaves, even though surviving Bob controlled it"
            );
            let count = queue.entries.len();
            for entry in &queue.entries {
                assert_eq!((entry.source, entry.controller), (observer, bob));
                assert!(
                    entry.source_snapshot.is_some(),
                    "Loss trigger retains predeparture source snapshot"
                );
            }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            results.push((count, game.player(bob).unwrap().life));
            assert_eq!(game.player(PlayerId::from_index(2)).unwrap().life, 20);
        }
    }
    assert_eq!(
        results,
        vec![(1, 23); 8],
        "CR 603.9/603.10f: all four loss causes trigger using the observer as it existed before departure, including a surviving controller's departed source"
    );
}

#[test]
fn cr_603_9_leaving_due_to_draw_does_not_trigger_player_loss() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::events::EventKind;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, Effect, Until, Zone};
    let alice = PlayerId::from_index(0);
    let carol = PlayerId::from_index(2);
    for owner in [alice, carol] {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Carol".into(),
                "Dave".into(),
                "Eve".into(),
            ],
            20,
        );
        game.enable_limited_range_of_influence(
            (0..5).map(PlayerId::from_index).collect(),
            vec![1, 1, 2, 1, 1],
        )
        .unwrap();
        assert!(!game.players_within_range(alice).contains(&carol));
        assert!(game.players_within_range(carol).contains(&alice));
        let definition = CardDefinitionBuilder::new(CardId::new(), "Draw departure observer")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::triggered(
                Trigger::player_loses_game(PlayerFilter::Specific(alice)),
                vec![Effect::gain_life(3)],
            ))
            .build();
        let observer = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
        if owner == alice {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(observer),
                Modification::ChangeController(carol),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(observer, carol))
            .unwrap();
        }
        assert_eq!(game.current_controller(observer), Some(carol));
        let mut expected_drawn = game.players_within_range(alice);
        expected_drawn.sort();
        assert_eq!(
            expected_drawn,
            vec![alice, PlayerId::from_index(1), PlayerId::from_index(4)]
        );
        let drawn = game.draw_game_for_controller_and_range(alice).unwrap();
        assert_eq!(drawn, expected_drawn);
        assert_eq!(drawn.len(), 3);
        assert!(drawn.contains(&alice));
        assert!(!drawn.contains(&carol));
        for player in drawn {
            assert!(game.player(player).unwrap().has_left_game);
            assert!(
                !game.player(player).unwrap().has_lost,
                "Draw is departure without losing"
            );
        }
        assert!(!game.player(carol).unwrap().has_left_game);
        assert_eq!(game.object(observer).is_none(), owner == alice);
        let events = game.take_pending_trigger_events();
        assert!(
            events
                .iter()
                .all(|event| event.kind() != EventKind::PlayerLosesGame),
            "CR 603.9: draw departures must not even publish player-loss events"
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.player(carol).unwrap().life, 20);
    }
}

#[test]
fn cr_603_10d_control_loss_and_opponent_gain_use_prechange_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::{ControlChangedTrigger, Trigger};
    use ironsmith::{Ability, CardId, CardType, Effect, ObjectFilter, Until, Zone};
    use ironsmith_core::trigger_model::ControlChangeDirection;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let filter = ObjectFilter::source().controlled_by(PlayerFilter::You);
    let mut definition = CardDefinitionBuilder::new(CardId::new(), "Control-transition observer")
        .card_types(vec![CardType::Artifact]);
    for change in [
        ControlChangeDirection::Lost {
            player: PlayerFilter::You,
        },
        ControlChangeDirection::Gained {
            player: PlayerFilter::You,
            from: None,
        },
        ControlChangeDirection::Gained {
            player: PlayerFilter::Opponent,
            from: Some(PlayerFilter::You),
        },
    ] {
        definition = definition.with_ability(Ability::triggered(
            Trigger::new(ControlChangedTrigger {
                filter: filter.clone(),
                change,
            }),
            vec![Effect::gain_life(1)],
        ));
    }
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let source = game.create_object_from_definition(&definition.build(), alice, Zone::Battlefield);
    let mut counts = Vec::new();
    let mut life_totals = Vec::new();
    for (stage, controller) in [alice, bob, carol, alice].into_iter().enumerate() {
        if stage == 3 {
            ironsmith::turn::execute_cleanup_step(&mut game);
        } else {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::ChangeController(controller),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        }
        assert_eq!(game.current_controller(source), Some(controller));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let mut by_player = [0; 3];
        for entry in &queue.entries {
            assert_eq!(entry.source, source);
            by_player[entry.controller.index()] += 1;
        }
        counts.push(by_player);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        life_totals.push([
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life,
            game.player(carol).unwrap().life,
        ]);
    }
    assert_eq!(
        counts,
        vec![[0, 0, 0], [2, 1, 0], [0, 2, 1], [1, 0, 2]],
        "CR 603.10d: loss and opponent-gains-from-you use old controller, ordinary gain uses new controller; expiry makes one net change"
    );
    assert_eq!(
        life_totals,
        vec![[20, 20, 20], [22, 21, 20], [22, 23, 21], [23, 23, 23]]
    );
}

#[test]
fn cr_603_10c_unattachment_uses_pretransition_source_and_recipient_characteristics() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::triggers::{AttachmentChangedTrigger, Trigger};
    use ironsmith::{Ability, CardId, CardType, Effect, ObjectFilter, Subtype, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for owner in [alice, bob] {
        for path in 0..4 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host = counter_creature(&mut game, Zone::Battlefield);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Unattachment observer Equipment")
                    .card_types(vec![CardType::Artifact])
                    .subtypes(vec![Subtype::Equipment])
                    .with_ability(Ability::triggered(
                        Trigger::new(AttachmentChangedTrigger {
                            attachment: ObjectFilter::source(),
                            recipient: ObjectFilter::creature(),
                            attached: false,
                        }),
                        vec![Effect::gain_life(3)],
                    ))
                    .build();
            let equipment =
                game.create_object_from_definition(&definition, owner, Zone::Battlefield);
            if owner == bob {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(equipment),
                    Modification::ChangeController(alice),
                    Until::EndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(equipment, alice))
                .unwrap();
            }
            assert_eq!(game.current_controller(equipment), Some(alice));
            assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(
                queue.entries.is_empty(),
                "Attachment itself cannot trigger unattachment"
            );
            match path {
                0 => {
                    assert!(game.detach_object_from_current_target(equipment));
                    assert!(!game.detach_object_from_current_target(equipment));
                }
                1 => {
                    game.move_object_by_effect(equipment, Zone::Graveyard)
                        .unwrap();
                }
                2 => {
                    game.move_object_by_effect(host, Zone::Graveyard).unwrap();
                }
                _ => {
                    ApplyContinuousEffect::new(
                        EffectTarget::Specific(host),
                        Modification::SetCardTypes(vec![CardType::Land]),
                        Until::EndOfTurn,
                    )
                    .execute(&mut game, &mut EffectContext::new_default(equipment, alice))
                    .unwrap();
                }
            }
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            if let Some(equipment) = game.object(equipment) {
                assert!(equipment.attached_to.is_none());
            }
            let count = queue.entries.len();
            for entry in &queue.entries {
                assert_eq!((entry.source, entry.controller), (equipment, alice));
            }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            results.push((count, game.player(alice).unwrap().life));
            assert_eq!(game.player(bob).unwrap().life, 20);
        }
    }
    let expected = vec![(1, 23), (1, 23), (1, 23), (0, 20)];
    assert_eq!(
        results,
        [expected.clone(), expected].concat(),
        "CR 603.10c: departing Equipment/host still trigger from pre-unattachment snapshots; host already made a land is no longer a creature before the SBA detaches"
    );
}

#[test]
fn cr_603_10b_phase_out_triggers_use_sources_before_direct_and_indirect_phasing() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::triggers::{AttachmentChangedTrigger, PhasingChangedTrigger, Trigger};
    use ironsmith::{Ability, CardId, CardType, CounterType, Effect, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for kind in 0..3 {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host_def = CardDefinitionBuilder::new(CardId::new(), "Phase-out look-back host")
                .card_types(vec![CardType::Creature, CardType::Land])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .with_ability(Ability::triggered(
                    Trigger::this_phases_out(),
                    vec![Effect::gain_life(1)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::new(PhasingChangedTrigger {
                        filter: ObjectFilter::permanent(),
                        phased_in: false,
                        one_or_more: true,
                    }),
                    vec![Effect::gain_life(10)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::this_enters_battlefield(),
                    vec![Effect::gain_life(1000)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::this_leaves_battlefield(),
                    vec![Effect::gain_life(1000)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::source_controller_loses_control("this permanent"),
                    vec![Effect::gain_life(1000)],
                ))
                .build();
            let host = game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
            let builder = CardDefinitionBuilder::new(CardId::new(), "Phasing attachment observer")
                .card_types(vec![if kind == 0 {
                    CardType::Enchantment
                } else {
                    CardType::Artifact
                }])
                .subtypes(vec![match kind {
                    0 => Subtype::Aura,
                    1 => Subtype::Equipment,
                    _ => Subtype::Fortification,
                }])
                .with_ability(Ability::triggered(
                    Trigger::this_phases_out(),
                    vec![Effect::gain_life(2)],
                ))
                .with_ability(Ability::triggered(
                    Trigger::new(AttachmentChangedTrigger {
                        attachment: ObjectFilter::source(),
                        recipient: ObjectFilter::permanent(),
                        attached: false,
                    }),
                    vec![Effect::gain_life(1000)],
                ));
            let attachment_def = if kind == 0 {
                builder.enchants(ObjectFilter::permanent())
            } else {
                builder
            }
            .build();
            let attachment =
                game.create_object_from_definition(&attachment_def, bob, Zone::Battlefield);
            assert!(game.attach_object_to_target(attachment, AttachmentTarget::Object(host)));
            game.add_counters(host, CounterType::PlusOnePlusOne, 1);
            game.add_counters(attachment, CounterType::Charge, 2);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(queue.entries.is_empty());
            let selected = match mode {
                0 => vec![host],
                1 => vec![attachment],
                _ => vec![attachment, host],
            };
            game.phase_out_simultaneously(&selected);
            assert_eq!(game.is_phased_out(host), mode != 1);
            assert!(game.is_phased_out(attachment));
            assert_eq!(
                game.object(attachment).unwrap().attached_to,
                Some(AttachmentTarget::Object(host))
            );
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            let mut controllers = [0; 2];
            for entry in &queue.entries {
                controllers[entry.controller.index()] += 1;
            }
            assert_eq!(
                controllers,
                if mode == 1 { [1, 1] } else { [2, 1] },
                "CR 603.10b: look-back sees phased-out host's own and batch observers plus phased-out attachment's own observer; kind={kind}, mode={mode}"
            );
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(
                (
                    game.player(alice).unwrap().life,
                    game.player(bob).unwrap().life
                ),
                (if mode == 1 { 30 } else { 31 }, 22)
            );
            game.phase_out_simultaneously(&selected);
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(
                queue.entries.is_empty(),
                "Repeated phase-out of already absent participants is not a new event"
            );
            game.phase_in(if mode == 1 { attachment } else { host });
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                queue.entries.is_empty(),
                "Phase-in is not ETB, control loss or unattachment"
            );
            assert!(!game.is_phased_out(host) && !game.is_phased_out(attachment));
            assert_eq!(game.object(host).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.object(attachment).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.counter_count(host, CounterType::PlusOnePlusOne), 1);
            assert_eq!(game.counter_count(attachment, CounterType::Charge), 2);
            assert_eq!(
                game.object(attachment).unwrap().attached_to,
                Some(AttachmentTarget::Object(host))
            );
        }
    }
}

#[test]
fn cr_702_26g_h_indirect_attachments_wait_for_their_host_even_with_their_own_phasing() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{Ability, CardId, CardType, ObjectFilter, StaticAbility, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for kind in 0..3 {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host_def = CardDefinitionBuilder::new(CardId::new(), "Alice phasing host")
                .card_types(vec![CardType::Creature, CardType::Land])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                .build();
            let host = game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
            let builder = CardDefinitionBuilder::new(CardId::new(), "Bob phasing attachment")
                .card_types(vec![if kind == 0 {
                    CardType::Enchantment
                } else {
                    CardType::Artifact
                }])
                .subtypes(vec![match kind {
                    0 => Subtype::Aura,
                    1 => Subtype::Equipment,
                    _ => Subtype::Fortification,
                }])
                .with_ability(Ability::static_ability(StaticAbility::phasing()));
            let attachment_def = if kind == 0 {
                builder.enchants(ObjectFilter::permanent())
            } else {
                builder
            }
            .build();
            let attachment =
                game.create_object_from_definition(&attachment_def, bob, Zone::Battlefield);
            assert!(game.attach_object_to_target(attachment, AttachmentTarget::Object(host)));
            game.tap(host);
            game.tap(attachment);
            game.phase_out_simultaneously(&match mode {
                0 => vec![host],
                1 => vec![attachment],
                _ => vec![attachment, host],
            });
            game.turn.active_player = bob;
            ironsmith::turn::execute_untap_step(&mut game);
            let indirect = mode != 1;
            assert_eq!(
                (game.is_phased_out(host), game.is_phased_out(attachment)),
                (indirect, indirect),
                "CR 702.26g-h: Bob's untap cannot return an indirectly phased-out attachment, even when it was also selected directly and has phasing"
            );
            assert_eq!(
                (game.is_tapped(host), game.is_tapped(attachment)),
                (true, indirect)
            );
            game.turn.active_player = alice;
            ironsmith::turn::execute_untap_step(&mut game);
            assert!(!game.is_phased_out(host) && !game.is_phased_out(attachment));
            assert_eq!(
                (game.is_tapped(host), game.is_tapped(attachment)),
                (false, indirect),
                "Returning with Alice's host does not untap Bob's attachment"
            );
            assert_eq!(
                game.object(attachment).unwrap().attached_to,
                Some(AttachmentTarget::Object(host))
            );
        }
    }
}

#[test]
fn cr_702_26b_i_phased_out_attachment_is_not_observably_unattached_when_host_leaves() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::triggers::{AttachmentChangedTrigger, Trigger};
    use ironsmith::{Ability, CardId, CardType, Effect, ObjectFilter, Subtype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for attachment_phased_out in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = counter_creature(&mut game, Zone::Battlefield);
        let trigger = Trigger::new(AttachmentChangedTrigger {
            attachment: ObjectFilter::artifact().with_subtype(Subtype::Equipment),
            recipient: ObjectFilter::creature(),
            attached: false,
        });
        let equipment_def =
            CardDefinitionBuilder::new(CardId::new(), "Temporarily absent Equipment")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![Subtype::Equipment])
                .with_ability(Ability::triggered(
                    trigger.clone(),
                    vec![Effect::gain_life(3)],
                ))
                .build();
        let equipment = game.create_object_from_definition(&equipment_def, bob, Zone::Battlefield);
        let observer_def = CardDefinitionBuilder::new(CardId::new(), "Unattachment event observer")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::triggered(trigger, vec![Effect::gain_life(5)]))
            .build();
        game.create_object_from_definition(&observer_def, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(equipment, AttachmentTarget::Object(host)));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        assert!(queue.entries.is_empty());
        if attachment_phased_out {
            game.phase_out(equipment);
            assert!(game.is_phased_out(equipment));
            assert!(!game.is_phased_out(host));
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(queue.entries.is_empty());
        }
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let host_departure_triggers = queue.entries.len();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        if attachment_phased_out {
            assert!(game.is_phased_out(equipment));
            game.phase_in(equipment);
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                queue.entries.is_empty(),
                "CR 702.26j: phase-in unattached must not itself produce an attachment-change trigger"
            );
        }
        assert!(game.object(equipment).unwrap().attached_to.is_none());
        assert!(!game.is_phased_out(equipment));
        results.push((
            host_departure_triggers,
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life,
        ));
    }
    assert_eq!(
        results,
        vec![(2, 25, 23), (0, 20, 20)],
        "CR 702.26b/i: absent Equipment cannot be observably affected when visible host leaves; it returns unattached, without unattachment triggers"
    );
}

#[test]
fn cr_603_10e_countered_spell_uses_stack_source_and_pre_counter_colors() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ColorSet, Effect, ObjectFilter, StackEntry, Until,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut results = Vec::new();
    for recolor_red in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let self_trigger = Ability::triggered(
            Trigger::spell_countered(
                Some(ObjectFilter::source().in_zone(Zone::Stack)),
                PlayerFilter::You,
            ),
            vec![Effect::gain_life(3)],
        )
        .in_zones(vec![Zone::Stack]);
        let spell_def = CardDefinitionBuilder::new(CardId::new(), "Self-observing countered spell")
            .card_types(vec![CardType::Sorcery])
            .color_indicator(ColorSet::BLUE)
            .with_spell_effect(vec![Effect::gain_life(1000)])
            .with_ability(self_trigger)
            .build();
        let spell = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
        game.stack.push(StackEntry::new(spell, alice));
        if recolor_red {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(spell),
                Modification::SetColors(ColorSet::RED),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(spell, alice))
            .unwrap();
        }
        assert_eq!(
            game.current_colors(spell).unwrap(),
            if recolor_red {
                ColorSet::RED
            } else {
                ColorSet::BLUE
            }
        );
        let observer_def = CardDefinitionBuilder::new(CardId::new(), "Countered-color observer")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::triggered(
                Trigger::spell_countered(
                    Some(ObjectFilter::spell().with_colors(ColorSet::RED)),
                    PlayerFilter::Any,
                ),
                vec![Effect::gain_life(5)],
            ))
            .with_ability(Ability::triggered(
                Trigger::spell_countered(
                    Some(ObjectFilter::spell().with_colors(ColorSet::BLUE)),
                    PlayerFilter::Any,
                ),
                vec![Effect::gain_life(7)],
            ))
            .build();
        let observer = game.create_object_from_definition(&observer_def, bob, Zone::Battlefield);
        game.stack.push(StackEntry::ability(
            observer,
            bob,
            vec![Effect::counter(ChooseSpec::SpecificObject(spell))],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert!(game.object(spell).is_none());
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        let card = game.player(alice).unwrap().graveyard[0];
        assert_eq!(
            game.current_colors(card).unwrap(),
            ColorSet::BLUE,
            "Zone change creates new card unaffected by old stack color change"
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        let mut controllers = [0; 2];
        for entry in &queue.entries {
            controllers[entry.controller.index()] += 1;
            assert!(entry.source_snapshot.is_some());
            if entry.controller == alice {
                assert_eq!(
                    entry.source, spell,
                    "Spell's own Stack-zone ability triggers from pre-counter source"
                );
            }
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        while !game.stack.is_empty() {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        results.push((
            controllers,
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life,
        ));
    }
    assert_eq!(
        results,
        vec![([1, 1], 23, 27), ([1, 1], 23, 25)],
        "CR 603.10e: self-observer is discovered before leaving stack and spell color is read before countering, never from new graveyard object"
    );
}

#[test]
fn cr_603_10e_countering_ability_does_not_publish_spell_countered_event() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Effect, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Countered ability source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let observer_def = CardDefinitionBuilder::new(CardId::new(), "Any-countered-spell observer")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::triggered(
            Trigger::spell_countered(None, PlayerFilter::Any),
            vec![Effect::gain_life(3)],
        ))
        .build();
    let observer = game.create_object_from_definition(&observer_def, bob, Zone::Battlefield);
    game.stack.push(StackEntry::ability(
        source,
        alice,
        vec![Effect::gain_life(1000)],
    ));
    game.stack.push(StackEntry::ability(
        observer,
        bob,
        vec![Effect::counter(ChooseSpec::SpecificObject(source))],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(game.stack.is_empty());
    assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        queue.entries.is_empty(),
        "Countering an ability is not a spell being countered"
    );
    assert_eq!(
        (
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life
        ),
        (20, 20)
    );
}

#[test]
fn cr_603_12_reflexive_trigger_checks_action_now_but_chooses_targets_after_parent_resolution() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::{DealDamageEffect, ReflexiveTriggerEffect};
    use ironsmith::{
        CardId, CardType, ChooseSpec, Effect, ObjectFilter, ObjectId, StackEntry, Target, Zone,
    };
    struct ReflexiveDecisions {
        accept: bool,
        parent_done: bool,
        target: ObjectId,
        optional_calls: usize,
        target_calls: usize,
    }
    impl DecisionMaker for ReflexiveDecisions {
        fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
            assert!(!self.parent_done);
            assert_eq!(ctx.player, PlayerId::from_index(0));
            self.optional_calls += 1;
            self.accept
        }
        fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            assert!(
                self.parent_done,
                "CR 603.12/603.3d: reflexive target choice waits until parent has finished resolving"
            );
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(ctx.requirements.len(), 1);
            assert_eq!(
                ctx.requirements[0].legal_targets,
                vec![Target::Object(self.target)]
            );
            self.target_calls += 1;
            vec![Target::Object(self.target)]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for accept in [false, true] {
        for target_leaves_before_parent_finishes in [false, true] {
            for source_leaves_before_parent_finishes in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source_def =
                    CardDefinitionBuilder::new(CardId::new(), "Reflexive parent source")
                        .card_types(vec![CardType::Artifact])
                        .build();
                let source =
                    game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
                let sacrifice = counter_creature(&mut game, Zone::Battlefield);
                let victim_def =
                    CardDefinitionBuilder::new(CardId::new(), "Reflexive target creature")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
                        .build();
                let target =
                    game.create_object_from_definition(&victim_def, bob, Zone::Battlefield);
                let target_spec = ChooseSpec::target_creature();
                let mut effects = vec![
                    Effect::with_id(
                        17,
                        Effect::may(vec![Effect::sacrifice(
                            ObjectFilter::creature().controlled_by(PlayerFilter::You),
                            1,
                        )]),
                    ),
                    Effect::new(ReflexiveTriggerEffect::new(
                        EffectId(17),
                        EffectPredicate::Happened,
                        vec![Effect::new(DealDamageEffect::new(2, target_spec.clone()))],
                        vec![target_spec],
                    )),
                ];
                if target_leaves_before_parent_finishes {
                    effects.push(Effect::exile(ChooseSpec::SpecificObject(target)));
                }
                if source_leaves_before_parent_finishes {
                    effects.push(Effect::exile(ChooseSpec::Source));
                }
                game.stack.push(StackEntry::ability(source, alice, effects));
                let mut dm = ReflexiveDecisions {
                    accept,
                    parent_done: false,
                    target,
                    optional_calls: 0,
                    target_calls: 0,
                };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert!(
                    game.stack.is_empty(),
                    "Reflexive trigger cannot be pushed as an immediate child resolution"
                );
                assert_eq!((dm.optional_calls, dm.target_calls), (1, 0));
                assert_eq!(game.object(sacrifice).is_none(), accept);
                assert_eq!(
                    game.object(target).is_none(),
                    target_leaves_before_parent_finishes
                );
                assert_eq!(
                    game.object(source).is_none(),
                    source_leaves_before_parent_finishes
                );
                if !target_leaves_before_parent_finishes {
                    assert_eq!(game.damage_on(target), 0);
                }
                dm.parent_done = true;
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                assert_eq!(
                    queue.entries.len(),
                    usize::from(accept),
                    "Only actual antecedent sacrifice triggers follow-up"
                );
                ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                    .unwrap();
                let can_stack = accept && !target_leaves_before_parent_finishes;
                assert_eq!(game.stack.len(), usize::from(can_stack));
                assert_eq!(dm.target_calls, usize::from(can_stack));
                if can_stack {
                    assert_eq!(
                        (game.stack[0].object_id, game.stack[0].controller),
                        (source, alice)
                    );
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(game.damage_on(target), 2);
                }
                assert_eq!(
                    (
                        game.player(alice).unwrap().life,
                        game.player(bob).unwrap().life
                    ),
                    (20, 20)
                );
            }
        }
    }
}

#[test]
fn cr_603_12_reflexive_antecedent_cannot_be_borrowed_from_an_earlier_resolution() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::{Condition, EffectId, EffectPredicate};
    use ironsmith::effects::ReflexiveTriggerEffect;
    use ironsmith::{CardId, CardType, Effect, ObjectFilter, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Reflexive resolution-scope source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let subject = counter_creature(&mut game, Zone::Battlefield);
    let antecedent = Effect::with_id(
        17,
        Effect::sacrifice(ObjectFilter::creature().controlled_by(PlayerFilter::You), 1),
    );
    game.stack
        .push(StackEntry::ability(source, alice, vec![antecedent.clone()]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert!(game.object(subject).is_none());
    game.stack.push(StackEntry::ability(
        source,
        alice,
        vec![
            Effect::conditional_only(Condition::LifeTotalOrGreater(30), vec![antecedent]),
            Effect::new(ReflexiveTriggerEffect::new(
                EffectId(17),
                EffectPredicate::Happened,
                vec![Effect::gain_life(3)],
                vec![],
            )),
        ],
    ));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        queue.entries.is_empty(),
        "CR 603.12: same numeric result ID in an earlier resolution cannot satisfy skipped antecedent in this resolution"
    );
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert!(game.stack.is_empty());
    assert_eq!(game.player(alice).unwrap().life, 20);
}

#[test]
fn cr_604_3_characteristic_defining_pt_color_and_subtype_update_in_every_zone() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{
        Ability, CardId, CardType, ColorSet, ObjectFilter, PowerToughness, PtValue, StaticAbility,
        Subtype, Until, Value, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for zone in [
        Zone::Library,
        Zone::Hand,
        Zone::Battlefield,
        Zone::Graveyard,
        Zone::Stack,
        Zone::Exile,
        Zone::Command,
        Zone::Ante,
        Zone::OutsideGame,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let filler = CardDefinitionBuilder::new(CardId::new(), "CDA hand-count filler")
            .card_types(vec![CardType::Artifact])
            .build();
        for _ in 0..2 {
            game.create_object_from_definition(&filler, alice, Zone::Hand);
        }
        for _ in 0..5 {
            game.create_object_from_definition(&filler, bob, Zone::Hand);
        }
        let count = Value::CardsInHand(PlayerFilter::You);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "All-zone characteristic source")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Zombie])
                .color_indicator(ColorSet::BLUE)
                .power_toughness(PowerToughness::new(PtValue::Star, PtValue::StarPlus(1)))
                .with_ability(Ability::static_ability(
                    StaticAbility::characteristic_defining_pt(
                        count.clone(),
                        Value::Add(Box::new(count), Box::new(Value::Fixed(1))),
                    ),
                ))
                .with_ability(Ability::static_ability(StaticAbility::set_colors(
                    ObjectFilter::source(),
                    ColorSet::RED,
                )))
                .with_ability(Ability::static_ability(StaticAbility::add_subtypes(
                    ObjectFilter::source(),
                    vec![Subtype::Elf],
                )))
                .build();
        let source = game.create_object_from_definition(&definition, alice, zone);
        if zone == Zone::Stack {
            // A spell cast by Bob from Alice's card retains Alice as owner.
            game.object_mut(source).unwrap().initial_controller = bob;
            game.stack.push(ironsmith::StackEntry::new(source, bob));
            assert_eq!(game.current_controller(source), Some(bob));
        }
        if zone == Zone::Battlefield {
            ApplyContinuousEffect::new(
                EffectTarget::Specific(source),
                Modification::ChangeController(bob),
                Until::EndOfTurn,
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            assert_eq!(game.current_controller(source), Some(bob));
        }
        for update in 0..3 {
            if update == 1 {
                game.create_object_from_definition(&filler, alice, Zone::Hand);
            }
            if update == 2 {
                game.create_object_from_definition(&filler, bob, Zone::Hand);
            }
            let referenced = if matches!(zone, Zone::Battlefield | Zone::Stack) {
                bob
            } else {
                alice
            };
            let expected = game.player(referenced).unwrap().hand.len() as i32;
            assert_eq!(
                (game.current_power(source), game.current_toughness(source)),
                (Some(expected), Some(expected + 1)),
                "CR 604.3/113.6a: zone={zone:?}, update={update}, You refers to controller where present and owner where absent"
            );
            assert_eq!(
                game.current_colors(source),
                Some(ColorSet::RED),
                "CDA color overrides printed blue even outside battlefield"
            );
            let subtypes = game.current_subtypes(source).unwrap();
            assert!(
                subtypes.contains(&Subtype::Zombie) && subtypes.contains(&Subtype::Elf),
                "Printed CDA adds subtype in zone {zone:?}"
            );
        }
    }
}

#[test]
fn cr_208_2a_undefined_cda_number_is_zero_even_inside_arithmetic() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{
        Ability, CardId, CardType, PowerToughness, PtValue, StaticAbility, Value, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut results = Vec::new();
    for zone in [
        Zone::Library,
        Zone::Hand,
        Zone::Battlefield,
        Zone::Graveyard,
        Zone::Stack,
        Zone::Exile,
        Zone::Command,
        Zone::Ante,
        Zone::OutsideGame,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        // Mirrors compiled CDAs: explicit zero fallback and exact linked choice acquisition.
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([93; 32]),
            pair: 0,
        };
        let undefined = Value::SourceChosenNumber {
            if_unset: Some(0),
            pair: Some(pair),
        };
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "CDA with not-yet-chosen number")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::new(PtValue::Star, PtValue::StarPlus(1)))
                .with_ability(Ability::static_ability(
                    StaticAbility::characteristic_defining_pt(
                        undefined.clone(),
                        Value::Add(Box::new(undefined), Box::new(Value::Fixed(1))),
                    ),
                ))
                .build();
        let source = game.create_object_from_definition(&definition, alice, zone);
        let owner = ironsmith::linked_exile::LinkedExileOwner {
            host: source,
            pair,
            acquisition: ironsmith::linked_exile::LinkedExileAcquisition::Printed,
        };
        for chosen in [None, Some(0), Some(7)] {
            if let Some(number) = chosen {
                game.set_number_for_acquisition(owner.clone(), number)
                    .unwrap();
            }
            results.push((game.current_power(source), game.current_toughness(source)));
        }
    }
    assert_eq!(
        results,
        vec![vec![(Some(0), Some(1)), (Some(0), Some(1)), (Some(7), Some(8))]; 9].concat(),
        "CR 208.2a: a CDA's undetermined number is zero as an operand; zero plus one is one in every zone, including outside game"
    );
}

#[test]
fn cr_604_4_attachment_static_bonuses_follow_host_without_targeting() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::AttachToEffect;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, ObjectFilter, StaticAbility, Subtype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for kind in 0..3 {
        for protection in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let mut host_builder = CardDefinitionBuilder::new(CardId::new(), "Static-bonus host")
                .card_types(vec![CardType::Creature, CardType::Land])
                .power_toughness(ironsmith::PowerToughness::fixed(3, 4));
            if protection != 0 {
                host_builder =
                    host_builder.with_ability(Ability::static_ability(if protection == 1 {
                        StaticAbility::shroud()
                    } else {
                        StaticAbility::hexproof()
                    }));
            }
            let host_def = host_builder.build();
            let first = game.create_object_from_definition(&host_def, alice, Zone::Battlefield);
            let second = game.create_object_from_definition(&host_def, bob, Zone::Battlefield);
            let attached = ObjectFilter::tagged(match kind {
                0 => "enchanted",
                1 => "equipped",
                _ => "fortified",
            });
            let builder = CardDefinitionBuilder::new(CardId::new(), "Following static attachment")
                .card_types(vec![if kind == 0 {
                    CardType::Enchantment
                } else {
                    CardType::Artifact
                }])
                .subtypes(vec![match kind {
                    0 => Subtype::Aura,
                    1 => Subtype::Equipment,
                    _ => Subtype::Fortification,
                }])
                .with_ability(Ability::static_ability(StaticAbility::anthem(
                    attached.clone(),
                    2,
                    3,
                )))
                .with_ability(Ability::static_ability(StaticAbility::grant_ability(
                    attached,
                    StaticAbility::flying(),
                )));
            let definition = if kind == 0 {
                builder.enchants(ObjectFilter::permanent())
            } else {
                builder
            }
            .build();
            let source = game.create_object_from_definition(&definition, carol, Zone::Battlefield);
            let check = |game: &GameState, active: Option<ironsmith::ObjectId>| {
                for host in [first, second] {
                    let boosted = Some(host) == active;
                    assert_eq!(
                        (game.current_power(host), game.current_toughness(host)),
                        if boosted {
                            (Some(5), Some(7))
                        } else {
                            (Some(3), Some(4))
                        },
                        "CR 604.4: kind={kind}, protection={protection}, active={active:?}, host={host:?}"
                    );
                    assert_eq!(
                        game.object_has_ability(host, &StaticAbility::flying()),
                        boosted
                    );
                }
                assert_eq!(
                    (
                        game.current_controller(first),
                        game.current_controller(second),
                        game.current_controller(source)
                    ),
                    (Some(alice), Some(bob), Some(carol))
                );
            };
            AttachToEffect::new(ChooseSpec::SpecificObject(first))
                .execute(&mut game, &mut EffectContext::new_default(source, carol))
                .unwrap();
            assert_eq!(
                game.object(source).unwrap().attached_to,
                Some(AttachmentTarget::Object(first))
            );
            check(&game, Some(first));
            AttachToEffect::new(ChooseSpec::SpecificObject(second))
                .execute(&mut game, &mut EffectContext::new_default(source, carol))
                .unwrap();
            assert_eq!(
                game.object(source).unwrap().attached_to,
                Some(AttachmentTarget::Object(second))
            );
            check(&game, Some(second));
            game.phase_out(source);
            assert!(!game.is_phased_out(second));
            check(&game, None);
            game.phase_in(source);
            assert_eq!(
                game.object(source).unwrap().attached_to,
                Some(AttachmentTarget::Object(second))
            );
            check(&game, Some(second));
        }
    }
}

#[test]
fn cr_604_2_static_source_zone_controller_and_ability_presence_control_its_effects() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Ability, CardId, CardType, ObjectFilter, StaticAbility, Until, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let host_def = CardDefinitionBuilder::new(CardId::new(), "Ordinary static-effect recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(3, 4))
        .build();
    let hosts = [alice, bob, carol]
        .map(|owner| game.create_object_from_definition(&host_def, owner, Zone::Battlefield));
    let definition = CardDefinitionBuilder::new(CardId::new(), "Ordinary static source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::anthem(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
            2,
            3,
        )))
        .build();
    let mut source = game.create_object_from_definition(&definition, carol, Zone::Hand);
    let check = |game: &GameState, boosted: Option<PlayerId>| {
        for (owner, host) in [alice, bob, carol].into_iter().zip(hosts) {
            assert_eq!(
                (game.current_power(host), game.current_toughness(host)),
                if Some(owner) == boosted {
                    (Some(5), Some(7))
                } else {
                    (Some(3), Some(4))
                },
                "CR 604.2: ordinary static effect follows current source, owner={owner:?}, boosted={boosted:?}"
            );
        }
        assert!(
            game.stack.is_empty(),
            "Static effects do not create or resolve stack entries"
        );
    };
    check(&game, None);
    source = game
        .move_object_by_effect(source, Zone::Battlefield)
        .unwrap();
    check(&game, Some(carol));
    for controller in [alice, bob] {
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::ChangeController(controller),
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, carol))
        .unwrap();
        assert_eq!(game.current_controller(source), Some(controller));
        check(&game, Some(controller));
    }
    ApplyContinuousEffect::new(
        EffectTarget::Specific(source),
        Modification::RemoveAllAbilities,
        Until::EndOfTurn,
    )
    .execute(&mut game, &mut EffectContext::new_default(source, carol))
    .unwrap();
    check(&game, None);
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert_eq!(game.current_controller(source), Some(carol));
    check(&game, Some(carol));
    source = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert_eq!(game.object(source).unwrap().zone, Zone::Graveyard);
    check(&game, None);
}

#[test]
fn cr_605_1a_mana_classification_crosses_targets_loyalty_library_movement_and_timing() {
    use ironsmith::ability::{ActivatedAbilityRuntimeExt, ActivationTiming};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effect::Condition;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, CounterType, Effect, ManaSymbol, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Mana-classification source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut checked = 0;
    for produces in [false, true] {
        for targeted in [false, true] {
            for loyalty in [false, true] {
                for library_cost in 0..3 {
                    for side_effect in 0..6 {
                        for timing in [
                            ActivationTiming::AnyTime,
                            ActivationTiming::AsInstant,
                            ActivationTiming::SorcerySpeed,
                            ActivationTiming::OncePerTurn,
                        ] {
                            for condition_currently_false in [false, true] {
                                let base = if loyalty {
                                    vec![Cost::add_counters(CounterType::Loyalty, 1)]
                                } else {
                                    vec![]
                                };
                                let mut milling = base.clone();
                                milling.push(Cost::mill(1));
                                let cost = match library_cost {
                                    0 => TotalCost::from_costs(base),
                                    1 => TotalCost::from_costs(milling),
                                    _ => TotalCost::one_of(vec![
                                        TotalCost::from_costs(base),
                                        TotalCost::from_costs(milling),
                                    ]),
                                };
                                let mut effects = vec![match side_effect {
                                    0 => Effect::gain_life(1),
                                    1 => Effect::draw(1),
                                    2 => Effect::mill(1),
                                    3 => Effect::shuffle_library(),
                                    4 => Effect::scry(1),
                                    _ => Effect::may(vec![Effect::draw(1)]),
                                }];
                                if produces {
                                    effects.push(Effect::add_mana(vec![ManaSymbol::Green]));
                                }
                                if targeted {
                                    effects.push(Effect::new(DealDamageEffect::new(
                                        1,
                                        ChooseSpec::target_player(),
                                    )));
                                }
                                let mut ability = Ability::activated(cost, effects);
                                let AbilityKind::Activated(ref mut activated) = ability.kind else {
                                    unreachable!()
                                };
                                // Match compiled declarations: mana-production metadata and explicit target choices.
                                activated.mana_output = produces.then_some(vec![]);
                                activated.choices = if targeted {
                                    vec![ChooseSpec::target_player()]
                                } else {
                                    vec![]
                                };
                                activated.timing = timing;
                                activated.activation_condition =
                                    Some(Condition::LifeTotalOrGreater(
                                        if condition_currently_false { 30 } else { 10 },
                                    ));
                                let expected = produces
                                    && !targeted
                                    && !loyalty
                                    && library_cost == 0
                                    && matches!(side_effect, 0 | 3 | 4);
                                assert_eq!(
                                    (
                                        activated.is_mana_ability(),
                                        activated.is_runtime_mana_ability(&game, source, alice)
                                    ),
                                    (expected, expected),
                                    "CR 605.1a/605.2: produces={produces}, targeted={targeted}, loyalty={loyalty}, library cost={library_cost}, effect={side_effect}, timing={timing:?}, condition blocked={condition_currently_false}"
                                );
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 1152);
}

#[test]
fn cr_605_3b_activated_mana_and_other_effects_resolve_without_touching_pending_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, Phase, StackEntry, Step, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareBlockers);
    let source_def = CardDefinitionBuilder::new(CardId::new(), "Immediate mana and life source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::mana_with_effects(
            TotalCost::free(),
            vec![
                Effect::add_mana(vec![ManaSymbol::Green]),
                Effect::gain_life(2),
            ],
        ))
        .build();
    let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
    let background_def = CardDefinitionBuilder::new(CardId::new(), "Pending stack source")
        .card_types(vec![CardType::Artifact])
        .build();
    let background = game.create_object_from_definition(&background_def, bob, Zone::Battlefield);
    game.stack.push(StackEntry::ability(
        background,
        bob,
        vec![Effect::gain_life(7)],
    ));
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform_activate_mana_ability(&mut game, alice, source, 0, &mut dm)
        .unwrap();
    assert!(game.is_tapped(source));
    assert_eq!(
        (
            game.player(alice).unwrap().mana_pool.green,
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life
        ),
        (1, 22, 20)
    );
    assert_eq!(game.stack.len(), 1);
    assert_eq!(
        (game.stack[0].object_id, game.stack[0].controller),
        (background, bob),
        "CR 605.3b: pending stack is untouched by immediate mana-ability resolution"
    );
    assert!(
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game, alice, source, 0, &mut dm
        )
        .is_err()
    );
    assert_eq!(
        (
            game.player(alice).unwrap().mana_pool.green,
            game.player(alice).unwrap().life
        ),
        (1, 22)
    );
    assert_eq!(game.stack.len(), 1);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 27);
}

#[test]
fn cr_605_3a_and_605_4a_payment_during_resolution_activates_mana_and_immediate_mana_triggers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaSymbol, ObjectFilter, ObjectId, Phase,
        StackEntry, Step, Zone,
    };
    struct PayInsideResolution {
        accept: bool,
        mana_source: ObjectId,
        background: ObjectId,
        offers: usize,
        payments: usize,
        red_before_confirmation: Option<u32>,
        resumed_plan_payable: Option<bool>,
    }
    impl DecisionMaker for PayInsideResolution {
        fn decide_boolean(&mut self, game: &GameState, ctx: &BooleanContext) -> bool {
            self.offers += 1;
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(game.turn.priority_player, None);
            self.accept
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.payments += 1;
            let alice = PlayerId::from_index(0);
            assert_eq!(ctx.player, alice);
            assert_eq!(
                game.turn.priority_player, None,
                "No priority is granted during the resolving effect's payment window"
            );
            assert_eq!(game.stack.len(), 1);
            assert_eq!(
                game.stack[0].object_id, self.background,
                "Mana ability and its mana trigger cannot be put onto the pending stack"
            );
            if self.payments == 1 {
                assert!(!game.is_tapped(self.mana_source));
                assert_eq!(
                    (
                        game.player(alice).unwrap().mana_pool.green,
                        game.player(alice).unwrap().mana_pool.red,
                        game.player(alice).unwrap().life
                    ),
                    (0, 0, 20)
                );
                ManaPaymentResponse::Activate {
                    source: self.mana_source,
                    ability_index: 0,
                }
            } else {
                assert_eq!(self.payments, 2);
                assert!(game.is_tapped(self.mana_source));
                assert_eq!(
                    (
                        game.player(alice).unwrap().mana_pool.green,
                        game.player(alice).unwrap().life
                    ),
                    (1, 22)
                );
                self.red_before_confirmation = Some(game.player(alice).unwrap().mana_pool.red);
                self.resumed_plan_payable = Some(ctx.plan.payable);
                if !ctx.plan.payable {
                    // Close the optional payment only after recording that legal funding is unavailable.
                    return ManaPaymentResponse::Cancel;
                }
                assert!(ctx.plan.mana_ability_steps.is_empty());
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    for accept in [false, true] {
        for bonus in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            game.turn.active_player = bob;
            game.turn.priority_player = None;
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareBlockers);
            let mana_def = CardDefinitionBuilder::new(CardId::new(), "Payment-window land")
                .card_types(vec![CardType::Land])
                .with_ability(Ability::mana_with_effects(
                    ironsmith::cost::TotalCost::free(),
                    vec![
                        Effect::add_mana(vec![ManaSymbol::Green]),
                        Effect::gain_life(2),
                    ],
                ))
                .build();
            let mana_source =
                game.create_object_from_definition(&mana_def, alice, Zone::Battlefield);
            let event = Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land());
            let mut observer_builder = CardDefinitionBuilder::new(
                CardId::new(),
                "Payment mana and ordinary trigger observer",
            )
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                event.clone(),
                vec![Effect::gain_life(3)],
            ));
            if bonus {
                observer_builder = observer_builder.with_ability(Ability::triggered(
                    event,
                    vec![Effect::add_mana(vec![ManaSymbol::Red])],
                ));
            }
            let observer = game.create_object_from_definition(
                &observer_builder.build(),
                alice,
                Zone::Battlefield,
            );
            if bonus {
                let abilities = game.current_abilities(observer).unwrap();
                let AbilityKind::Triggered(ref mana_trigger) = abilities[1].kind else {
                    unreachable!()
                };
                assert!(mana_trigger.choices.is_empty());
                assert!(
                    mana_trigger
                        .effects
                        .iter()
                        .any(Effect::contains_mana_production)
                );
            }
            let other_def =
                CardDefinitionBuilder::new(CardId::new(), "Payment parent and background source")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let parent = game.create_object_from_definition(&other_def, bob, Zone::Battlefield);
            let background =
                game.create_object_from_definition(&other_def, carol, Zone::Battlefield);
            game.stack.push(StackEntry::ability(
                background,
                carol,
                vec![Effect::gain_life(7)],
            ));
            let mut cost = vec![ManaSymbol::Green];
            if bonus {
                cost.push(ManaSymbol::Red);
            }
            game.stack.push(StackEntry::ability(
                parent,
                bob,
                vec![Effect::new(UnlessPaysEffect::new(
                    vec![Effect::gain_life(5)],
                    PlayerFilter::Specific(alice),
                    cost,
                ))],
            ));
            let mut dm = PayInsideResolution {
                accept,
                mana_source,
                background,
                offers: 0,
                payments: 0,
                red_before_confirmation: None,
                resumed_plan_payable: None,
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                dm.resumed_plan_payable,
                accept.then_some(true),
                "CR 605.4a: after manual activation, full cost must be fundable with triggered red mana; bonus={bonus}, red before confirmation={:?}",
                dm.red_before_confirmation
            );
            assert_eq!((dm.offers, dm.payments), (1, if accept { 2 } else { 0 }));
            assert_eq!(game.is_tapped(mana_source), accept);
            assert_eq!(
                (
                    game.player(alice).unwrap().mana_pool.green,
                    game.player(alice).unwrap().mana_pool.red
                ),
                (0, 0)
            );
            assert_eq!(
                (
                    game.player(alice).unwrap().life,
                    game.player(bob).unwrap().life,
                    game.player(carol).unwrap().life
                ),
                (
                    if accept { 22 } else { 20 },
                    if accept { 20 } else { 25 },
                    20
                )
            );
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.stack[0].object_id, background);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert_eq!(
                queue.entries.len(),
                usize::from(accept),
                "Only ordinary +3 life trigger remains pending; mana trigger already resolved during payment"
            );
            for entry in &queue.entries {
                assert_eq!((entry.source, entry.controller), (observer, alice));
            }
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            assert_eq!(game.stack.len(), if accept { 2 } else { 1 });
            if accept {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(alice).unwrap().life, 25);
            }
            assert_eq!(game.stack[0].object_id, background);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(carol).unwrap().life, 27);
        }
    }
}

#[test]
fn cr_605_5b_mana_producing_spell_uses_normal_stack_and_can_be_countered() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ManaSymbol, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for counter in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let definition = CardDefinitionBuilder::new(CardId::new(), "Mana-producing sorcery")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::add_mana(vec![ManaSymbol::Green; 3])])
            .build();
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        game.stack.push(StackEntry::new(spell, alice));
        assert!(!game.stack[0].is_ability);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            0,
            "A mana-producing spell waits on stack for resolution"
        );
        if counter {
            let counter_def =
                CardDefinitionBuilder::new(CardId::new(), "Countering source for mana spell")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let source = game.create_object_from_definition(&counter_def, bob, Zone::Battlefield);
            game.stack.push(StackEntry::ability(
                source,
                bob,
                vec![Effect::counter(ChooseSpec::SpecificObject(spell))],
            ));
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.stack.is_empty());
        assert!(game.object(spell).is_none());
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            if counter { 0 } else { 3 },
            "CR 605.5b: normal spell resolution produces mana, countered spell produces none"
        );
    }
}

#[test]
fn cr_605_1b_and_605_5a_triggered_mana_classification_controls_stack_and_counterability() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::TargetsContext;
    use ironsmith::effects::{DealDamageEffect, MoveToZoneEffect};
    use ironsmith::triggers::Trigger;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, ManaSymbol, ObjectFilter,
        StackEntry, Target, Zone,
    };
    struct TargetBob {
        calls: usize,
    }
    impl DecisionMaker for TargetBob {
        fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            self.calls += 1;
            assert_eq!(ctx.player, PlayerId::from_index(0));
            assert_eq!(ctx.requirements.len(), 1);
            assert!(
                ctx.requirements[0]
                    .legal_targets
                    .contains(&Target::Player(PlayerId::from_index(1)))
            );
            assert_eq!(
                (
                    game.player(PlayerId::from_index(0)).unwrap().mana_pool.red,
                    game.player(PlayerId::from_index(0)).unwrap().life
                ),
                (0, 20)
            );
            vec![Target::Player(PlayerId::from_index(1))]
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut checked = 0;
    for from_mana_event in [false, true] {
        for produces in [false, true] {
            for targeted in [false, true] {
                for counter in [false, true] {
                    let should_be_mana = from_mana_event && produces && !targeted;
                    if should_be_mana && counter {
                        continue;
                    }
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.priority_player = Some(alice);
                    game.turn.phase = ironsmith::Phase::FirstMain;
                    game.turn.step = None;
                    let land_def = CardDefinitionBuilder::new(
                        CardId::new(),
                        "Triggered-mana classification land",
                    )
                    .card_types(vec![CardType::Land])
                    .with_ability(Ability::mana(
                        ironsmith::cost::TotalCost::free(),
                        vec![ManaSymbol::Green],
                    ))
                    .build();
                    let land =
                        game.create_object_from_definition(&land_def, alice, Zone::Battlefield);
                    let trigger = if from_mana_event {
                        Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land())
                    } else {
                        Trigger::this_enters_battlefield()
                    };
                    let mut body = vec![if produces {
                        Effect::add_mana(vec![ManaSymbol::Red])
                    } else {
                        Effect::gain_life(3)
                    }];
                    if targeted {
                        body.push(Effect::new(DealDamageEffect::new(
                            0,
                            ChooseSpec::target_player(),
                        )));
                    }
                    let mut ability = Ability::triggered(trigger, body);
                    if targeted {
                        let AbilityKind::Triggered(ref mut triggered) = ability.kind else {
                            unreachable!()
                        };
                        triggered.choices = vec![ChooseSpec::target_player()];
                    }
                    let definition = CardDefinitionBuilder::new(
                        CardId::new(),
                        "Triggered classification observer",
                    )
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(ability)
                    .build();
                    let mut observer = game.create_object_from_definition(
                        &definition,
                        alice,
                        if from_mana_event {
                            Zone::Battlefield
                        } else {
                            Zone::Graveyard
                        },
                    );
                    let mut dm = TargetBob { calls: 0 };
                    if from_mana_event {
                        ironsmith::special_actions::perform_activate_mana_ability(
                            &mut game, alice, land, 0, &mut dm,
                        )
                        .unwrap();
                    } else {
                        let stable = game.object(observer).unwrap().stable_id;
                        MoveToZoneEffect::new(
                            ChooseSpec::SpecificObject(observer),
                            Zone::Battlefield,
                            false,
                        )
                        .execute(&mut game, &mut EffectContext::new_default(land, alice))
                        .unwrap();
                        observer = game
                            .battlefield
                            .iter()
                            .copied()
                            .find(|id| {
                                game.object(*id)
                                    .is_some_and(|object| object.stable_id == stable)
                            })
                            .unwrap();
                    }
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                    if !should_be_mana {
                        assert_eq!(queue.entries.len(), 1);
                    }
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(
                        &mut game, &mut queue, &mut dm,
                    )
                    .unwrap();
                    assert_eq!(dm.calls, usize::from(targeted));
                    assert_eq!(
                        game.stack.len(),
                        usize::from(!should_be_mana),
                        "CR 605.1b/605.5a: event mana={from_mana_event}, produces={produces}, target={targeted}"
                    );
                    if should_be_mana {
                        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
                        assert_eq!(game.player(alice).unwrap().life, 20);
                    } else {
                        assert_eq!(
                            (
                                game.player(alice).unwrap().mana_pool.red,
                                game.player(alice).unwrap().life
                            ),
                            (0, 20)
                        );
                        assert_eq!(
                            (game.stack[0].object_id, game.stack[0].controller),
                            (observer, alice)
                        );
                        if targeted {
                            assert_eq!(game.stack[0].targets, vec![Target::Player(bob)]);
                        }
                        if counter {
                            let source_def = CardDefinitionBuilder::new(
                                CardId::new(),
                                "Trigger-countering source",
                            )
                            .card_types(vec![CardType::Artifact])
                            .build();
                            let source = game.create_object_from_definition(
                                &source_def,
                                bob,
                                Zone::Battlefield,
                            );
                            game.stack.push(StackEntry::ability(
                                source,
                                bob,
                                vec![Effect::counter(ChooseSpec::SpecificObject(observer))],
                            ));
                        }
                        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                        assert!(game.stack.is_empty());
                        assert!(
                            game.object(observer).is_some(),
                            "Countering ability leaves its permanent source in place"
                        );
                        assert_eq!(
                            game.player(alice).unwrap().mana_pool.red,
                            u32::from(produces && !counter)
                        );
                        assert_eq!(
                            game.player(alice).unwrap().life,
                            if !produces && !counter { 23 } else { 20 }
                        );
                    }
                    assert_eq!(
                        game.player(alice).unwrap().mana_pool.green,
                        u32::from(from_mana_event)
                    );
                    assert_eq!(game.player(bob).unwrap().life, 20);
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 15);
}

#[test]
fn cr_605_3a_and_605_3c_other_mana_ability_on_same_source_can_pay_activation_cost() {
    use ironsmith::StackEntry;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, ObjectId, Zone,
    };
    struct FundActivation {
        source: ObjectId,
        funder: ObjectId,
        index: usize,
        calls: usize,
    }
    impl DecisionMaker for FundActivation {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            if self.calls == 1 {
                ManaPaymentResponse::Activate {
                    source: self.source,
                    ability_index: 0,
                }
            } else if self.calls == 2 {
                assert_eq!(ctx.source, self.source);
                let choices = ironsmith::mana_payment::manual_mana_abilities(game, &ctx.request);
                assert!(
                    !choices.contains(&(self.source, 0)),
                    "CR 605.3c forbids reentry of the ability whose activation is underway"
                );
                assert!(choices.contains(&(self.funder, self.index)));
                ManaPaymentResponse::Activate {
                    source: self.funder,
                    ability_index: self.index,
                }
            } else {
                assert!(matches!(self.calls, 3 | 4));
                assert!(ctx.plan.payable);
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            }
        }
    }
    let alice = PlayerId::from_index(0);
    for same_source in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut converter = Ability::activated(
            TotalCost::from_cost(Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Green]))),
            vec![Effect::add_mana(vec![ManaSymbol::Green, ManaSymbol::Green])],
        );
        let AbilityKind::Activated(ref mut activated) = converter.kind else {
            unreachable!()
        };
        activated.mana_output = Some(vec![]);
        let funding = Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]);
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Mana converter")
            .card_types(vec![CardType::Artifact])
            .with_ability(converter);
        if same_source {
            builder = builder.with_ability(funding.clone());
        }
        let source = game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        let (funder, index) = if same_source {
            (source, 1)
        } else {
            let definition = CardDefinitionBuilder::new(CardId::new(), "Separate funding source")
                .card_types(vec![CardType::Artifact])
                .with_ability(funding)
                .build();
            (
                game.create_object_from_definition(&definition, alice, Zone::Battlefield),
                0,
            )
        };
        let parent_definition =
            CardDefinitionBuilder::new(CardId::new(), "Resolving mana-payment instruction")
                .card_types(vec![CardType::Artifact])
                .build();
        let parent = game.create_object_from_definition(
            &parent_definition,
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        game.turn.priority_player = None;
        game.stack.push(StackEntry::ability(
            parent,
            PlayerId::from_index(1),
            vec![Effect::new(UnlessPaysEffect::new(
                vec![Effect::gain_life(5)],
                PlayerFilter::Specific(alice),
                vec![ManaSymbol::Green, ManaSymbol::Green],
            ))],
        ));
        let mut dm = FundActivation {
            source,
            funder,
            index,
            calls: 0,
        };
        let result = ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm);
        assert!(result.is_ok(), "Resolution failed: {result:?}");
        assert_eq!(
            dm.calls, 4,
            "CR 605.3a/605.3c: another mana ability can fund this activation; same source={same_source}"
        );
        assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
        assert!(game.is_tapped(funder));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_605_3a_mana_ability_can_be_activated_during_payment_already_funded_from_pool() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::{ManaPaymentResponse, manual_mana_abilities};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectId, StackEntry, Zone};
    struct FloatExtraMana {
        source: ObjectId,
        initial_green: u32,
        calls: usize,
    }
    impl DecisionMaker for FloatExtraMana {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            assert_eq!(game.turn.priority_player, None);
            if self.calls == 1 {
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool.green,
                    self.initial_green
                );
                assert!(ctx.plan.payable);
                assert!(
                    manual_mana_abilities(game, &ctx.request).contains(&(self.source, 0)),
                    "CR 605.3a: being asked for a mana payment permits activating mana abilities even when floating mana already covers the cost"
                );
                ManaPaymentResponse::Activate {
                    source: self.source,
                    ability_index: 0,
                }
            } else {
                assert_eq!(self.calls, 2);
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool.green,
                    self.initial_green + 1
                );
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for initial_green in [0, 1] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = bob;
        game.turn.priority_player = None;
        game.player_mut(alice).unwrap().mana_pool.green = initial_green;
        let definition = CardDefinitionBuilder::new(CardId::new(), "Extra mana source")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.stack.push(StackEntry::ability(
            source,
            bob,
            vec![Effect::new(UnlessPaysEffect::new(
                vec![Effect::gain_life(5)],
                PlayerFilter::Specific(alice),
                vec![ManaSymbol::Green],
            ))],
        ));
        let mut dm = FloatExtraMana {
            source,
            initial_green,
            calls: 0,
        };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, 2);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, initial_green);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn cr_605_3c_completed_mana_ability_can_be_activated_again() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut ability = Ability::activated(
        TotalCost::from_cost(Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Green]))),
        vec![Effect::add_mana(vec![ManaSymbol::Green, ManaSymbol::Green])],
    );
    let AbilityKind::Activated(ref mut activated) = ability.kind else {
        unreachable!()
    };
    activated.mana_output = Some(vec![]);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Repeatable mana converter")
        .card_types(vec![CardType::Artifact])
        .with_ability(ability)
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.player_mut(alice).unwrap().mana_pool.green = 1;
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    for expected_green in [2, 3, 4] {
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game, alice, source, 0, &mut dm,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.green, expected_green);
        assert!(!game.is_tapped(source));
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_605_2_zero_output_mana_ability_still_resolves_immediately_with_other_effects() {
    use ironsmith::ability::ActivatedAbilityRuntimeExt;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaSymbol, ObjectFilter, StackEntry, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for creatures in [0, 1, 3] {
        for initially_tapped in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.priority_player = Some(alice);
            for _ in 0..creatures {
                counter_creature(&mut game, Zone::Battlefield);
            }
            let observer_definition =
                CardDefinitionBuilder::new(CardId::new(), "Actual mana production observer")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::triggered(
                        ironsmith::triggers::Trigger::player_taps_for_mana(
                            PlayerFilter::You,
                            ObjectFilter::permanent(),
                        ),
                        vec![Effect::gain_life(3)],
                    ))
                    .build();
            let observer =
                game.create_object_from_definition(&observer_definition, alice, Zone::Battlefield);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Conditional quantity mana source")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::mana_with_effects(
                        TotalCost::free(),
                        vec![
                            Effect::new(ironsmith::effects::AddScaledManaEffect::new(
                                vec![ManaSymbol::Green],
                                ironsmith::effect::Value::Count(
                                    ObjectFilter::creature().controlled_by(PlayerFilter::You),
                                ),
                                PlayerFilter::You,
                            )),
                            Effect::gain_life(2),
                        ],
                    ))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let ability = game.current_ability(source, 0).unwrap();
            let AbilityKind::Activated(ref activated) = ability.kind else {
                unreachable!()
            };
            assert!(activated.is_mana_ability());
            assert!(activated.is_runtime_mana_ability(&game, source, alice));
            game.stack
                .push(StackEntry::ability(source, bob, vec![Effect::gain_life(5)]));
            if initially_tapped {
                game.tap(source);
            }
            let result = ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            );
            assert_eq!(result.is_ok(), !initially_tapped);
            assert!(game.is_tapped(source));
            assert_eq!(
                game.player(alice).unwrap().mana_pool.green,
                if initially_tapped { 0 } else { creatures }
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                if initially_tapped { 20 } else { 22 }
            );
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.stack.len(), 1);
            assert_eq!(
                (game.stack[0].object_id, game.stack[0].controller),
                (source, bob)
            );
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut game,
                &mut queue,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let triggers = !initially_tapped && creatures > 0;
            assert_eq!(
                game.stack.len(),
                1 + usize::from(triggers),
                "CR 106.12a: tapping for a zero-output mana ability does not trigger tapped-for-mana abilities"
            );
            if triggers {
                assert_eq!(game.stack.last().unwrap().object_id, observer);
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(alice).unwrap().life, 25);
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(bob).unwrap().life, 25);
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_605_3a_payment_can_activate_currently_zero_output_mana_ability_for_other_effects() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::{ManaPaymentResponse, manual_mana_abilities};
    use ironsmith::{
        Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, ObjectId, StackEntry, Zone,
    };
    struct UseSideEffect {
        source: ObjectId,
        funder: ObjectId,
        creatures: u32,
        calls: usize,
    }
    impl DecisionMaker for UseSideEffect {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            assert_eq!(game.turn.priority_player, None);
            if self.calls == 1 {
                assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, 0);
                assert!(
                    manual_mana_abilities(game, &ctx.request).contains(&(self.source, 0)),
                    "CR 605.3a/605.2: current zero mana output does not remove permission to activate a mana ability for its other effects"
                );
                ManaPaymentResponse::Activate {
                    source: self.source,
                    ability_index: 0,
                }
            } else if self.calls == 2 && self.creatures == 0 {
                assert_eq!(game.player(ctx.player).unwrap().life, 22);
                assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, 0);
                ManaPaymentResponse::Activate {
                    source: self.funder,
                    ability_index: 0,
                }
            } else {
                assert_eq!(self.calls, if self.creatures == 0 { 3 } else { 2 });
                assert_eq!(game.player(ctx.player).unwrap().life, 22);
                assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, 1);
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for creatures in [1, 0] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.priority_player = None;
        for _ in 0..creatures {
            counter_creature(&mut game, Zone::Battlefield);
        }
        let definition = CardDefinitionBuilder::new(CardId::new(), "Mana and life source")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana_with_effects(
                TotalCost::free(),
                vec![
                    Effect::new(ironsmith::effects::AddScaledManaEffect::new(
                        vec![ManaSymbol::Green],
                        ironsmith::effect::Value::Count(
                            ObjectFilter::creature().controlled_by(PlayerFilter::You),
                        ),
                        PlayerFilter::You,
                    )),
                    Effect::gain_life(2),
                ],
            ))
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let funder_definition = CardDefinitionBuilder::new(CardId::new(), "Backup payment source")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
            .build();
        let funder =
            game.create_object_from_definition(&funder_definition, alice, Zone::Battlefield);
        game.stack.push(StackEntry::ability(
            source,
            bob,
            vec![Effect::new(UnlessPaysEffect::new(
                vec![Effect::gain_life(5)],
                PlayerFilter::Specific(alice),
                vec![ManaSymbol::Green],
            ))],
        ));
        let mut dm = UseSideEffect {
            source,
            funder,
            creatures,
            calls: 0,
        };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.calls, if creatures == 0 { 3 } else { 2 });
        assert!(game.is_tapped(source));
        assert_eq!(game.is_tapped(funder), creatures == 0);
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
}

#[test]
fn cr_605_1b_mana_added_by_spell_triggers_mana_ability_without_using_stack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for green_amount in [0, 1, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let observer_definition = CardDefinitionBuilder::new(CardId::new(), "Mana-added observers")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::mana_added(PlayerFilter::You),
                vec![Effect::add_mana_player(
                    vec![ManaSymbol::Red],
                    PlayerFilter::Specific(bob),
                )],
            ))
            .with_ability(Ability::triggered(
                Trigger::mana_added(PlayerFilter::You),
                vec![Effect::gain_life(2)],
            ))
            .build();
        let observer =
            game.create_object_from_definition(&observer_definition, alice, Zone::Battlefield);
        let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Mana-producing sorcery")
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::add_mana(vec![
                ManaSymbol::Green;
                green_amount
            ])])
            .build();
        let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
        let stable_spell = game.object(spell).unwrap().stable_id;
        game.stack.push(StackEntry::ability(
            observer,
            bob,
            vec![Effect::gain_life(5)],
        ));
        game.stack.push(StackEntry::new(spell, alice));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            green_amount as u32
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.stack.len(), 1);
        assert!(game.player(alice).unwrap().graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable_spell)
        }));
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(queue.is_empty());
        assert_eq!(
            game.player(bob).unwrap().mana_pool.red,
            u32::from(green_amount > 0),
            "CR 605.1b: triggering from mana addition does not require the producer to be an activated mana ability"
        );
        assert_eq!(game.stack.len(), 1 + usize::from(green_amount > 0));
        assert_eq!(game.player(alice).unwrap().life, 20);
        if green_amount > 0 {
            assert_eq!(
                (
                    game.stack.last().unwrap().object_id,
                    game.stack.last().unwrap().controller
                ),
                (observer, alice)
            );
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 22);
        }
        assert_eq!(game.player(bob).unwrap().life, 20);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 25);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_106_10_and_106_11_generic_and_snow_output_symbols_add_colorless_mana() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::{Ability, CardId, CardType, Effect, ManaCost, ManaSymbol, Supertype, Zone};
    let alice = PlayerId::from_index(0);
    let mut mismatches = Vec::new();
    for snow_source in [false, true] {
        for (symbols, expected_colorless) in [
            (vec![ManaSymbol::Colorless], 1),
            (vec![ManaSymbol::Generic(0)], 0),
            (vec![ManaSymbol::Generic(1)], 1),
            (vec![ManaSymbol::Generic(3)], 3),
            (vec![ManaSymbol::Snow], 1),
            (vec![ManaSymbol::Snow, ManaSymbol::Snow], 2),
            (vec![ManaSymbol::Generic(2), ManaSymbol::Snow], 3),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Symbol-to-mana source")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::add_mana(symbols.clone())],
                ));
            if snow_source {
                builder = builder.supertypes(vec![Supertype::Snow]);
            }
            let source =
                game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let pool = &game.player(alice).unwrap().mana_pool;
            let actual_pool = (
                pool.white,
                pool.blue,
                pool.black,
                pool.red,
                pool.green,
                pool.colorless,
            );
            let expected_pool = (0, 0, 0, 0, 0, expected_colorless);
            let can_pay_snow = game.can_pay_mana_cost_with_reason(
                alice,
                Some(source),
                &ManaCost::from_symbols(vec![ManaSymbol::Snow]),
                0,
                PaymentReason::ActivateAbility,
            );
            let expected_snow = snow_source && expected_colorless > 0;
            if (actual_pool, can_pay_snow) != (expected_pool, expected_snow) {
                mismatches.push((
                    symbols,
                    snow_source,
                    actual_pool,
                    expected_pool,
                    can_pay_snow,
                    expected_snow,
                ));
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }
    assert!(
        mismatches.is_empty(),
        "CR 106.10/106.11: output symbols must credit colorless mana; snow spending depends on source supertype. Mismatches: {mismatches:?}"
    );
}

#[test]
fn cr_106_12a_one_tap_mana_activation_triggers_once_with_multiple_production_instructions() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    for separate_instructions in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let tapped_definition = CardDefinitionBuilder::new(CardId::new(), "Tap-for-mana observer")
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::triggered(
                Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::permanent()),
                vec![Effect::gain_life(3)],
            ))
            .build();
        let tapped_observer =
            game.create_object_from_definition(&tapped_definition, alice, Zone::Battlefield);
        let addition_definition =
            CardDefinitionBuilder::new(CardId::new(), "Each mana addition observer")
                .card_types(vec![CardType::Enchantment])
                .with_ability(Ability::triggered(
                    Trigger::mana_added(PlayerFilter::You),
                    vec![Effect::gain_life(5)],
                ))
                .build();
        let addition_observer =
            game.create_object_from_definition(&addition_definition, alice, Zone::Battlefield);
        let effects = if separate_instructions {
            vec![
                Effect::add_mana(vec![ManaSymbol::Green]),
                Effect::add_mana(vec![ManaSymbol::Red]),
            ]
        } else {
            vec![Effect::add_mana(vec![ManaSymbol::Green, ManaSymbol::Red])]
        };
        let source_definition =
            CardDefinitionBuilder::new(CardId::new(), "One activation with two mana units")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana_with_effects(TotalCost::free(), effects))
                .build();
        let source =
            game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(game.is_tapped(source));
        assert_eq!(
            (
                game.player(alice).unwrap().mana_pool.green,
                game.player(alice).unwrap().mana_pool.red
            ),
            (1, 1)
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.stack.is_empty());
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(
            queue
                .entries
                .iter()
                .filter(|entry| entry.source == addition_observer)
                .count(),
            if separate_instructions { 2 } else { 1 },
            "Actual mana additions remain distinct when the ability contains two instructions"
        );
        assert_eq!(
            queue
                .entries
                .iter()
                .filter(|entry| entry.source == tapped_observer)
                .count(),
            1,
            "CR 106.12a: one activation/resolution with one tap cost is one tapped-for-mana occurrence, separate instructions={separate_instructions}"
        );
    }
}

#[test]
fn cr_106_5_copy_without_originals_chosen_color_produces_no_mana() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::effects::{AddManaOfChosenColorEffect, CreateTokenCopyEffect};
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Color, Effect, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Chosen-color mana source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::mana_with_effects(
            TotalCost::free(),
            vec![
                Effect::new(AddManaOfChosenColorEffect::new(1, PlayerFilter::You)),
                Effect::gain_life(2),
            ],
        ))
        .build();
    let original = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_color(original, Color::Blue);
    ironsmith::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        original,
        0,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
    assert_eq!(game.player(alice).unwrap().life, 22);
    CreateTokenCopyEffect::one(ChooseSpec::SpecificObject(original))
        .execute(&mut game, &mut EffectContext::new_default(original, alice))
        .unwrap();
    let copy = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
        .unwrap();
    assert_eq!(game.chosen_color(copy), None);
    assert!(!game.is_tapped(copy));
    ironsmith::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        copy,
        0,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.is_tapped(copy));
    assert_eq!(
        game.player(alice).unwrap().life,
        24,
        "Other effects of the zero-output mana ability still resolve"
    );
    let pool = &game.player(alice).unwrap().mana_pool;
    assert_eq!(
        (
            pool.white,
            pool.blue,
            pool.black,
            pool.red,
            pool.green,
            pool.colorless
        ),
        (0, 1, 0, 0, 0, 0),
        "CR 106.5/707.6: an unchosen color is undefined; the copy cannot substitute green or inherit blue"
    );
    assert!(game.stack.is_empty());
}

#[test]
fn cr_106_5_commander_identity_without_colors_produces_no_mana() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::effects::AddManaFromCommanderColorIdentityEffect;
    use ironsmith::{Ability, CardId, CardType, ColorSet, Effect, PowerToughness, Supertype, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for identity in [Some(ColorSet::BLUE), Some(ColorSet::COLORLESS), None] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        if let Some(colors) = identity {
            let commander_definition =
                CardDefinitionBuilder::new(CardId::new(), "Identity control commander")
                    .card_types(vec![CardType::Creature])
                    .supertypes(vec![Supertype::Legendary])
                    .power_toughness(PowerToughness::fixed(1, 1))
                    .color_indicator(colors)
                    .build();
            let commander =
                game.create_object_from_definition(&commander_definition, alice, Zone::Command);
            game.set_as_commander(commander, alice);
        }
        assert_eq!(
            game.get_commander_color_identity(alice),
            identity.unwrap_or(ColorSet::COLORLESS)
        );
        let source_definition =
            CardDefinitionBuilder::new(CardId::new(), "Commander identity mana source")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddManaFromCommanderColorIdentityEffect::you(1))],
                ))
                .build();
        let source =
            game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert!(game.is_tapped(source));
        let pool = &game.player(alice).unwrap().mana_pool;
        actual.push((
            pool.white,
            pool.blue,
            pool.black,
            pool.red,
            pool.green,
            pool.colorless,
        ));
        expected.push((0, u32::from(identity == Some(ColorSet::BLUE)), 0, 0, 0, 0));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual, expected,
        "CR 106.5: commander identity supplies a color or no defined type; it never substitutes colorless"
    );
}

#[test]
fn cr_106_7_could_produce_includes_abilities_that_are_not_mana_abilities() {
    use ironsmith::ability::ActivatedAbilityRuntimeExt;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::AddManaOfLandProducedTypesEffect;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for draws_card in [false, true] {
        for tapped in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut effects = vec![Effect::add_mana(vec![ManaSymbol::Green])];
            if draws_card {
                effects.push(Effect::draw(1));
            }
            let ability = Ability::activated(TotalCost::from_cost(Cost::tap()), effects);
            let definition = CardDefinitionBuilder::new(
                CardId::new(),
                "Land producing mana with optional library movement",
            )
            .card_types(vec![CardType::Land])
            .with_ability(ability)
            .build();
            let land = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            if tapped {
                game.tap(land);
            }
            let current = game.current_ability(land, 0).unwrap();
            let AbilityKind::Activated(ref activated) = current.kind else {
                unreachable!()
            };
            assert_eq!(
                activated.is_runtime_mana_ability(&game, land, bob),
                !draws_card
            );
            let probe_definition = CardDefinitionBuilder::new(CardId::new(), "Could-produce probe")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddManaOfLandProducedTypesEffect::new(
                        1,
                        PlayerFilter::You,
                        ObjectFilter::land().controlled_by(PlayerFilter::Specific(bob)),
                        false,
                        true,
                    ))],
                ))
                .build();
            let probe =
                game.create_object_from_definition(&probe_definition, alice, Zone::Battlefield);
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                probe,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            actual.push(game.player(alice).unwrap().mana_pool.green);
            assert_eq!(game.is_tapped(land), tapped);
            assert!(
                game.player(bob).unwrap().hand.is_empty(),
                "The hypothetical producer is not actually resolved"
            );
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual,
        vec![1, 1, 1, 1],
        "CR 106.7 refers to what an ability would produce if it resolved, including a non-mana ability; costs need not be payable"
    );
}

#[test]
fn cr_106_7_could_produce_respects_current_zero_output_quantity() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::effect::Value;
    use ironsmith::effects::{AddManaOfLandProducedTypesEffect, AddScaledManaEffect};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for creatures in [1, 0] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        if creatures > 0 {
            counter_creature(&mut game, Zone::Battlefield);
        }
        let land_definition =
            CardDefinitionBuilder::new(CardId::new(), "Currently scaled mana land")
                .card_types(vec![CardType::Land])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddScaledManaEffect::new(
                        vec![ManaSymbol::Green],
                        Value::Count(
                            ObjectFilter::creature().controlled_by(PlayerFilter::Specific(alice)),
                        ),
                        PlayerFilter::You,
                    ))],
                ))
                .build();
        let land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);
        // Resolve the producer as a control before asking what it could produce now.
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            bob,
            land,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(bob).unwrap().mana_pool.green, creatures);
        let probe_definition = CardDefinitionBuilder::new(CardId::new(), "Current quantity probe")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana_with_effects(
                TotalCost::free(),
                vec![Effect::new(AddManaOfLandProducedTypesEffect::new(
                    1,
                    PlayerFilter::You,
                    ObjectFilter::land().controlled_by(PlayerFilter::Specific(bob)),
                    false,
                    true,
                ))],
            ))
            .build();
        let probe = game.create_object_from_definition(&probe_definition, alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            probe,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        actual.push(game.player(alice).unwrap().mana_pool.green);
        assert!(game.is_tapped(land));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![1, 0],
        "CR 106.7: ignore unpayable tap cost, but evaluate the output quantity in the current game state"
    );
}

#[test]
fn cr_106_7_mutual_orchards_propagate_a_real_mana_type_through_the_other_land() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::effects::AddManaOfLandProducedTypesEffect;
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for forest_owner in [None, Some(bob), Some(alice)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let orchard_definition =
            CardDefinitionBuilder::new(CardId::new(), "Opponent-land color source")
                .card_types(vec![CardType::Land])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddManaOfLandProducedTypesEffect::new(
                        1,
                        PlayerFilter::You,
                        ObjectFilter::land().controlled_by(PlayerFilter::Opponent),
                        false,
                        true,
                    ))],
                ))
                .build();
        let alice_orchard =
            game.create_object_from_definition(&orchard_definition, alice, Zone::Battlefield);
        let bob_orchard =
            game.create_object_from_definition(&orchard_definition, bob, Zone::Battlefield);
        if let Some(owner) = forest_owner {
            let forest_definition = CardDefinitionBuilder::new(CardId::new(), "Green seed land")
                .card_types(vec![CardType::Land])
                .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
                .build();
            game.create_object_from_definition(&forest_definition, owner, Zone::Battlefield);
        }
        if forest_owner == Some(alice) {
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                bob,
                bob_orchard,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(
                game.player(bob).unwrap().mana_pool.green,
                1,
                "Bob's orchard can produce green from Alice's actual green source"
            );
            assert!(game.is_tapped(bob_orchard));
        }
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            alice_orchard,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        actual.push(game.player(alice).unwrap().mana_pool.green);
        assert!(game.is_tapped(alice_orchard));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![0, 1, 1],
        "CR 106.7's explicit mutual-Orchard example: no seed means no color, but a green seed on either side supplies green to both Orchards"
    );
}

#[test]
fn cr_106_7_could_produce_applies_current_mana_replacement_effects() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::effects::AddManaOfLandProducedTypesEffect;
    use ironsmith::events::mana::matchers::ManaProducedBySourceMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for recolored in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let land_definition = CardDefinitionBuilder::new(CardId::new(), "Replaceable green land")
            .card_types(vec![CardType::Land])
            .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
            .build();
        let land = game.create_object_from_definition(&land_definition, bob, Zone::Battlefield);
        let probe_definition =
            CardDefinitionBuilder::new(CardId::new(), "Replacement-aware type probe")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddManaOfLandProducedTypesEffect::new(
                        1,
                        PlayerFilter::You,
                        ObjectFilter::land().controlled_by(PlayerFilter::Specific(bob)),
                        false,
                        true,
                    ))],
                ))
                .build();
        let probe = game.create_object_from_definition(&probe_definition, alice, Zone::Battlefield);
        if recolored {
            let mut filter = ObjectFilter::land();
            filter.specific = Some(land);
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    probe,
                    alice,
                    ManaProducedBySourceMatcher::new(filter),
                    ReplacementAction::ReplaceMana(vec![ManaSymbol::Blue]),
                ));
        }
        // A real resolution proves the current replacement affects precisely the other land.
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            bob,
            land,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            (
                game.player(bob).unwrap().mana_pool.green,
                game.player(bob).unwrap().mana_pool.blue
            ),
            if recolored { (0, 1) } else { (1, 0) }
        );
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            probe,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        actual.push((
            game.player(alice).unwrap().mana_pool.green,
            game.player(alice).unwrap().mana_pool.blue,
        ));
        assert!(game.is_tapped(land));
        assert!(game.is_tapped(probe));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![(1, 0), (0, 1)],
        "CR 106.7 requires could-produce types after applicable replacements, even though the land is now tapped"
    );
}

fn cr_audit_transfer_all_mana(game: &mut GameState, from: PlayerId, to: PlayerId) {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::EffectId;
    use ironsmith::effect::{EffectMetric, EffectMetricSource, PriorEffectMetricQuery, Value};
    use ironsmith::effects::{AddManaLostThisWayEffect, EmptyManaPoolEffect};
    use ironsmith::{CardId, CardType, Effect, StackEntry, Zone};
    let amount = Value::PriorEffectMetric {
        effect_id: EffectId(581),
        query: PriorEffectMetricQuery::new(EffectMetricSource::Outcome, EffectMetric::Count)
            .with_action(ironsmith_core::PriorEffectAction::ManaLost),
    };
    let definition = CardDefinitionBuilder::new(CardId::new(), "Transfer the lost mana")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![
            Effect::with_id(
                581,
                Effect::new(EmptyManaPoolEffect::new(PlayerFilter::Specific(from))),
            ),
            Effect::new(AddManaLostThisWayEffect::new(
                amount,
                PlayerFilter::Specific(to),
            )),
        ])
        .build();
    let spell = game.create_object_from_definition(&definition, to, Zone::Stack);
    game.stack.push(StackEntry::new(spell, to));
    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
    assert!(game.stack.is_empty());
}

#[test]
fn cr_106_13_transferred_mana_preserves_its_spending_restrictions() {
    use ironsmith::ability::ManaUsageRestriction;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, ManaCost, ManaSymbol, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for restricted in [false, true] {
        for recipient in [alice, bob] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
            if restricted {
                let AbilityKind::Activated(ref mut activated) = ability.kind else {
                    unreachable!()
                };
                activated.mana_usage_restrictions = vec![ManaUsageRestriction::CastSpellMatching {
                    filter: ironsmith::ObjectFilter::creature(),
                    restrict_to_matching_spell: true,
                    grant_uncounterable: false,
                    enters_with_counters: vec![],
                    granted_abilities: vec![],
                }];
            }
            let source_definition =
                CardDefinitionBuilder::new(CardId::new(), "Restricted production control")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(ability)
                    .build();
            let source =
                game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                bob,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.player(bob).unwrap().mana_pool.red, 1);
            assert_eq!(
                game.player(bob).unwrap().restricted_mana.len(),
                usize::from(restricted)
            );
            let cost = ManaCost::from_symbols(vec![ManaSymbol::Red]);
            let mut probes = Vec::new();
            for kind in [CardType::Creature, CardType::Instant] {
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Mana spending type probe")
                        .card_types(vec![kind])
                        .mana_cost(cost.clone())
                        .build();
                probes.push(game.create_object_from_definition(
                    &definition,
                    recipient,
                    Zone::Stack,
                ));
            }
            assert!(game.can_pay_mana_cost_with_reason(
                bob,
                Some(probes[0]),
                &cost,
                0,
                PaymentReason::CastSpell
            ));
            assert_eq!(
                game.can_pay_mana_cost_with_reason(
                    bob,
                    Some(probes[1]),
                    &cost,
                    0,
                    PaymentReason::CastSpell
                ),
                !restricted
            );
            cr_audit_transfer_all_mana(&mut game, bob, recipient);
            assert_eq!(game.player(recipient).unwrap().mana_pool.red, 1);
            if recipient != bob {
                assert_eq!(game.player(bob).unwrap().mana_pool.red, 0);
            }
            actual.push((
                game.can_pay_mana_cost_with_reason(
                    recipient,
                    Some(probes[0]),
                    &cost,
                    0,
                    PaymentReason::CastSpell,
                ),
                game.can_pay_mana_cost_with_reason(
                    recipient,
                    Some(probes[1]),
                    &cost,
                    0,
                    PaymentReason::CastSpell,
                ),
                game.player(recipient).unwrap().restricted_mana.len(),
            ));
            expected.push((true, !restricted, usize::from(restricted)));
        }
    }
    assert_eq!(
        actual, expected,
        "CR 106.13: moving the mana to another player or back to the same player must retain spending restrictions"
    );
}

#[test]
fn cr_106_13_transferred_mana_keeps_its_original_producer_and_snow_provenance() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::mana::{ManaProducerFilter, ManaSpendingRestriction};
    use ironsmith::{Ability, CardId, CardType, ManaCost, ManaSymbol, Supertype, Zone};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for snow_source in [false, true] {
        for recipient in [alice, bob] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut builder =
                CardDefinitionBuilder::new(CardId::new(), "Original artifact mana producer")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]));
            if snow_source {
                builder = builder.supertypes(vec![Supertype::Snow]);
            }
            let source =
                game.create_object_from_definition(&builder.build(), bob, Zone::Battlefield);
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                bob,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let snow_cost = ManaCost::from_symbols(vec![ManaSymbol::Snow]);
            let artifact_cost = ManaCost::from_symbols(vec![ManaSymbol::Red])
                .with_spending_restriction(ManaSpendingRestriction::ProducedBy(
                    ManaProducerFilter::CardType(CardType::Artifact),
                ));
            let probe_definition =
                CardDefinitionBuilder::new(CardId::new(), "Producer-restricted spell probe")
                    .card_types(vec![CardType::Instant])
                    .build();
            let probe =
                game.create_object_from_definition(&probe_definition, recipient, Zone::Stack);
            assert_eq!(
                game.can_pay_mana_cost_with_reason(
                    bob,
                    Some(probe),
                    &snow_cost,
                    0,
                    PaymentReason::CastSpell
                ),
                snow_source
            );
            assert!(game.can_pay_mana_cost_with_reason(
                bob,
                Some(probe),
                &artifact_cost,
                0,
                PaymentReason::CastSpell
            ));
            // Existing mana keeps the production snapshot even after its source leaves.
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            assert!(game.can_pay_mana_cost_with_reason(
                bob,
                Some(probe),
                &artifact_cost,
                0,
                PaymentReason::CastSpell
            ));
            cr_audit_transfer_all_mana(&mut game, bob, recipient);
            assert_eq!(game.player(recipient).unwrap().mana_pool.red, 1);
            if recipient != bob {
                assert_eq!(game.player(bob).unwrap().mana_pool.red, 0);
            }
            actual.push((
                game.can_pay_mana_cost_with_reason(
                    recipient,
                    Some(probe),
                    &snow_cost,
                    0,
                    PaymentReason::CastSpell,
                ),
                game.can_pay_mana_cost_with_reason(
                    recipient,
                    Some(probe),
                    &artifact_cost,
                    0,
                    PaymentReason::CastSpell,
                ),
            ));
            expected.push((snow_source, true));
        }
    }
    assert_eq!(
        actual, expected,
        "CR 106.13: Drain Power transfers the existing mana's source properties; its sorcery is not the new producer"
    );
}

#[test]
fn cr_106_6_doubled_mana_is_new_mana_without_original_restrictions_or_snow() {
    use ironsmith::ability::ManaUsageRestriction;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::effects::DoubleManaPoolEffect;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, Supertype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut original_source = None;
    for restricted in [true, false] {
        let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
        if restricted {
            let AbilityKind::Activated(ref mut activated) = ability.kind else {
                unreachable!()
            };
            activated.mana_usage_restrictions = vec![ManaUsageRestriction::CastSpellMatching {
                filter: ironsmith::ObjectFilter::creature(),
                restrict_to_matching_spell: true,
                grant_uncounterable: false,
                enters_with_counters: vec![],
                granted_abilities: vec![],
            }];
        }
        let mut builder = CardDefinitionBuilder::new(CardId::new(), "Original mana for doubling")
            .card_types(vec![CardType::Artifact])
            .with_ability(ability);
        if restricted {
            builder = builder.supertypes(vec![Supertype::Snow]);
        }
        let source = game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        if restricted {
            original_source = Some(source);
        }
    }
    assert_eq!(game.player(alice).unwrap().mana_pool.red, 2);
    let original_restricted = game.player(alice).unwrap().restricted_mana.clone();
    assert_eq!(original_restricted.len(), 1);
    assert_eq!(original_restricted[0].source, original_source.unwrap());
    let cube_definition = CardDefinitionBuilder::new(CardId::new(), "New mana from doubling")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::mana_with_effects(
            TotalCost::free(),
            vec![Effect::new(DoubleManaPoolEffect::you())],
        ))
        .build();
    let cube = game.create_object_from_definition(&cube_definition, alice, Zone::Battlefield);
    ironsmith::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        cube,
        0,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
    .unwrap();
    assert_eq!(game.player(alice).unwrap().mana_pool.red, 4);
    assert_eq!(
        game.player(alice).unwrap().restricted_mana,
        original_restricted
    );
    let mut probes = Vec::new();
    for kind in [CardType::Creature, CardType::Instant] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Doubled mana spending probe")
            .card_types(vec![kind])
            .build();
        probes.push(game.create_object_from_definition(&definition, alice, Zone::Stack));
    }
    let payable = |source, symbols| {
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(source),
            &ManaCost::from_symbols(symbols),
            0,
            PaymentReason::CastSpell,
        )
    };
    assert!(payable(probes[0], vec![ManaSymbol::Red; 4]));
    assert!(
        payable(probes[1], vec![ManaSymbol::Red; 3]),
        "New doubled mana is unrestricted"
    );
    assert!(
        !payable(probes[1], vec![ManaSymbol::Red; 4]),
        "The old restricted unit stays restricted"
    );
    assert!(payable(probes[0], vec![ManaSymbol::Snow]));
    assert!(
        !payable(probes[0], vec![ManaSymbol::Snow; 2]),
        "New units are produced by the nonsnow doubling source"
    );
    assert!(
        !payable(probes[1], vec![ManaSymbol::Snow]),
        "Only the original restricted unit came from a snow source"
    );
    assert!(game.is_tapped(cube));
    assert!(game.stack.is_empty());
}

#[test]
fn cr_106_6a_mana_increased_by_replacement_keeps_restrictions_on_every_unit() {
    use ironsmith::ability::ManaUsageRestriction;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::events::mana::matchers::ManaProducedBySourceMatcher;
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, Zone,
    };
    let alice = PlayerId::from_index(0);
    for multiplier in [1, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let restriction = ManaUsageRestriction::CastSpellMatching {
            filter: ObjectFilter::creature(),
            restrict_to_matching_spell: true,
            grant_uncounterable: false,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        };
        let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.mana_usage_restrictions = vec![restriction.clone()];
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Restricted mana with multiplied production")
                .card_types(vec![CardType::Artifact])
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if multiplier != 1 {
            let mut filter = ObjectFilter::permanent();
            filter.specific = Some(source);
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    source,
                    alice,
                    ManaProducedBySourceMatcher::new(filter),
                    ReplacementAction::Modify(EventModification::Multiply(multiplier)),
                ));
        }
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.red, multiplier as u32);
        assert_eq!(
            game.player(alice).unwrap().restricted_mana.len(),
            multiplier as usize
        );
        for unit in &game.player(alice).unwrap().restricted_mana {
            assert_eq!(unit.source, source);
            assert_eq!(unit.symbol, ManaSymbol::Red);
            assert_eq!(unit.restrictions, vec![restriction.clone()]);
        }
        let mut probes = Vec::new();
        for kind in [CardType::Creature, CardType::Instant] {
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Multiplied mana spending probe")
                    .card_types(vec![kind])
                    .build();
            probes.push(game.create_object_from_definition(&definition, alice, Zone::Stack));
        }
        assert!(game.can_pay_mana_cost_with_reason(
            alice,
            Some(probes[0]),
            &ManaCost::from_symbols(vec![ManaSymbol::Red; multiplier as usize]),
            0,
            PaymentReason::CastSpell
        ));
        assert!(!game.can_pay_mana_cost_with_reason(
            alice,
            Some(probes[1]),
            &ManaCost::from_symbols(vec![ManaSymbol::Red]),
            0,
            PaymentReason::CastSpell
        ));
        assert!(!game.can_pay_mana_cost_with_reason(
            alice,
            Some(source),
            &ManaCost::from_symbols(vec![ManaSymbol::Red]),
            0,
            PaymentReason::ActivateAbility
        ));
    }
}

#[test]
fn cr_106_13_transferred_mana_keeps_its_uncounterable_spell_bonus() {
    use ironsmith::ability::{ManaSpendBonusCondition, ManaUsageRestriction};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ChooseSpec, Effect, ManaCost, ManaSymbol,
        ObjectFilter, StackEntry, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for (transfer, payer) in [(false, bob), (true, bob), (true, alice)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.mana_usage_restrictions = vec![ManaUsageRestriction::CastSpellWithManaBonus {
            filter: ObjectFilter::creature(),
            condition: ManaSpendBonusCondition::IfThisManaIsSpentToCast,
            grant_uncounterable: true,
            enters_with_counters: vec![],
            granted_abilities: vec![],
            granted_keywords: vec![],
            enters_with_counter_values: vec![],
        }];
        let source_definition =
            CardDefinitionBuilder::new(CardId::new(), "Mana carrying an uncounterable bonus")
                .card_types(vec![CardType::Artifact])
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            bob,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        if transfer {
            cr_audit_transfer_all_mana(&mut game, bob, payer);
        }
        let cost = ManaCost::from_symbols(vec![ManaSymbol::Red]);
        let creature_definition =
            CardDefinitionBuilder::new(CardId::new(), "Creature paid with bonus mana")
                .card_types(vec![CardType::Creature])
                .mana_cost(cost.clone())
                .build();
        let creature = game.create_object_from_definition(&creature_definition, payer, Zone::Stack);
        assert!(
            game.try_pay_mana_cost_with_reason(
                payer,
                Some(creature),
                &cost,
                0,
                PaymentReason::CastSpell
            )
            .unwrap()
        );
        assert_eq!(game.player(payer).unwrap().mana_pool.red, 0);
        game.stack.push(StackEntry::new(creature, payer));
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![Effect::counter(ChooseSpec::SpecificObject(creature))],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let survived = game
            .object(creature)
            .is_some_and(|object| object.zone == Zone::Stack)
            && game.stack.iter().any(|entry| entry.object_id == creature);
        actual.push(survived);
    }
    assert_eq!(
        actual,
        vec![true, true, true],
        "CR 106.13: original mana-spending bonus must survive a self-transfer or transfer to another player; countering the paid creature still fails"
    );
}

#[test]
fn cr_603_7e_spent_mana_delayed_trigger_uses_activator_not_permanent_controller() {
    use ironsmith::ability::{
        ActivationTiming, ManaPaymentPredicate, ManaSpendPayload, ManaUsageRestriction,
    };
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::decision::LegalAction;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, Phase, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut actual = Vec::new();
    for controller in [alice, bob] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.timing = ActivationTiming::AnyPlayerDuringTheirTurnBeforeEndStep;
        activated.mana_usage_restrictions = vec![ManaUsageRestriction::PaymentTransaction {
            restriction: None,
            on_spend: vec![ManaSpendPayload {
                predicate: ManaPaymentPredicate::Any,
                effects: vec![Effect::gain_life(2)].into(),
                choices: vec![],
            }],
        }];
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Mana ability available to either player")
                .card_types(vec![CardType::Artifact])
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
        let actions = ironsmith::decision::compute_legal_actions(&game, alice).unwrap();
        assert!(actions.iter().any(|action| matches!(action, LegalAction::ActivateManaAbility { source: id, .. } if *id == source)),
            "Alice is legally allowed to activate the other player's ability during her turn");
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(bob).unwrap().mana_pool.red, 0);
        assert_eq!(
            game.player(alice).unwrap().restricted_mana[0].source_controller,
            Some(alice)
        );
        let cost = ManaCost::from_symbols(vec![ManaSymbol::Red]);
        let spell_definition =
            CardDefinitionBuilder::new(CardId::new(), "Spell consuming delayed-trigger mana")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(cost.clone())
                .build();
        let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
        assert!(
            game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell),
                &cost,
                0,
                PaymentReason::CastSpell
            )
            .unwrap()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.entries[0].source, source);
        let trigger_controller = queue.entries[0].controller;
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(game.stack.len(), 1);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        actual.push((
            trigger_controller,
            game.player(alice).unwrap().life,
            game.player(bob).unwrap().life,
        ));
    }
    assert_eq!(
        actual,
        vec![(alice, 22, 20); 2],
        "CR 603.7e: Alice controlled the activated mana ability that created the delayed trigger, even when Bob controls its permanent source"
    );
}

#[test]
fn cr_106_6a_increased_mana_creates_one_delayed_spending_trigger_per_unit() {
    use ironsmith::ability::{
        ManaPaymentPredicate, ManaPaymentPurpose, ManaSpendPayload, ManaUsageRestriction,
    };
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::events::mana::matchers::ManaProducedBySourceMatcher;
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, ObjectFilter, Zone,
    };
    let alice = PlayerId::from_index(0);
    for multiplier in [1, 3] {
        for matching_spell in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
            let AbilityKind::Activated(ref mut activated) = ability.kind else {
                unreachable!()
            };
            activated.mana_usage_restrictions = vec![ManaUsageRestriction::PaymentTransaction {
                restriction: None,
                on_spend: vec![ManaSpendPayload {
                    predicate: ManaPaymentPredicate::All(vec![
                        ManaPaymentPredicate::Purpose(ManaPaymentPurpose::CastSpell),
                        ManaPaymentPredicate::SourceMatches(ObjectFilter::creature()),
                    ]),
                    effects: vec![Effect::gain_life(2)].into(),
                    choices: vec![],
                }],
            }];
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Per-unit delayed mana bonus source")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(ability)
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if multiplier != 1 {
                let mut filter = ObjectFilter::permanent();
                filter.specific = Some(source);
                game.effect_store
                    .replacement_effects
                    .add_effect(ReplacementEffect::with_matcher(
                        source,
                        alice,
                        ManaProducedBySourceMatcher::new(filter),
                        ReplacementAction::Modify(EventModification::Multiply(multiplier)),
                    ));
            }
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let cost = ManaCost::from_symbols(vec![ManaSymbol::Red; multiplier as usize]);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Spell triggering per-unit mana payload")
                    .card_types(vec![if matching_spell {
                        CardType::Creature
                    } else {
                        CardType::Instant
                    }])
                    .mana_cost(cost.clone())
                    .build();
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            assert!(
                game.try_pay_mana_cost_with_reason(
                    alice,
                    Some(spell),
                    &cost,
                    0,
                    PaymentReason::CastSpell
                )
                .unwrap()
            );
            assert_eq!(game.player(alice).unwrap().mana_pool.red, 0);
            assert_eq!(
                game.player(alice).unwrap().life,
                20,
                "Spending triggers wait for the normal trigger boundary"
            );
            assert!(game.stack.is_empty());
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            let count = if matching_spell {
                multiplier as usize
            } else {
                0
            };
            assert_eq!(queue.entries.len(), count);
            assert!(
                queue
                    .entries
                    .iter()
                    .all(|entry| entry.source == source && entry.controller == alice)
            );
            ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut game,
                &mut queue,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.stack.len(), count);
            for _ in 0..count {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 20 + 2 * count as i32);
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_106_6a_increased_mana_applies_entry_counter_bonus_once_per_spent_unit() {
    use ironsmith::ability::{ManaSpendBonusCondition, ManaUsageRestriction};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::events::mana::matchers::ManaProducedBySourceMatcher;
    use ironsmith::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, CounterType, ManaCost, ManaSymbol, ObjectFilter,
        PowerToughness, StackEntry, Zone,
    };
    let alice = PlayerId::from_index(0);
    for multiplier in [1, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Red]);
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.mana_usage_restrictions = vec![ManaUsageRestriction::CastSpellWithManaBonus {
            filter: ObjectFilter::creature(),
            condition: ManaSpendBonusCondition::IfThisManaIsSpentToCast,
            grant_uncounterable: false,
            enters_with_counters: vec![(CounterType::PlusOnePlusOne, 1)],
            granted_abilities: vec![],
            granted_keywords: vec![],
            enters_with_counter_values: vec![],
        }];
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Per-unit entry-counter mana source")
                .card_types(vec![CardType::Artifact])
                .with_ability(ability)
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if multiplier != 1 {
            let mut filter = ObjectFilter::permanent();
            filter.specific = Some(source);
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    source,
                    alice,
                    ManaProducedBySourceMatcher::new(filter),
                    ReplacementAction::Modify(EventModification::Multiply(multiplier)),
                ));
        }
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        let cost = ManaCost::from_symbols(vec![ManaSymbol::Red; multiplier as usize]);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Creature receiving each mana bonus")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .mana_cost(cost.clone())
                .build();
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let stable = game.object(spell).unwrap().stable_id;
        assert!(
            game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell),
                &cost,
                0,
                PaymentReason::CastSpell
            )
            .unwrap()
        );
        game.stack.push(StackEntry::new(spell, alice));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let creature = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.stable_id == stable)
            })
            .unwrap();
        assert_eq!(
            game.counter_count(creature, CounterType::PlusOnePlusOne),
            multiplier as u32
        );
        assert_eq!(game.calculated_power(creature), Some(2 + multiplier as i32));
        assert_eq!(
            game.calculated_toughness(creature),
            Some(2 + multiplier as i32)
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 0);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_107_1b_negative_power_is_kept_for_calculations_but_zero_for_a_pump_amount() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::cost::TotalCost;
    use ironsmith::effect::Value;
    use ironsmith::effects::{
        AddScaledManaEffect, ApplyContinuousEffect, ModifyPowerToughnessEffect,
    };
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, ManaSymbol, PowerToughness, StackEntry,
        StaticAbility, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for power_reduction in [-6, -4, -2, 0] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Signed power calculation source")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(4, 4))
                .with_ability(Ability::static_ability(StaticAbility::haste()))
                .with_ability(Ability::mana_with_effects(
                    TotalCost::free(),
                    vec![Effect::new(AddScaledManaEffect::new(
                        vec![ManaSymbol::Green],
                        Value::SourcePower,
                        PlayerFilter::You,
                    ))],
                ))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::ModifyPowerToughness {
                power: power_reduction,
                toughness: 0,
            },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        let power = 4 + power_reduction;
        let usable = power.max(0);
        assert_eq!(game.current_power(source), Some(power));
        assert_eq!(game.current_toughness(source), Some(4));
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            1,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            usable as u32,
            "Negative power remains a game value, but cannot produce negative mana"
        );
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![
                Effect::gain_life(Value::Add(
                    Box::new(Value::SourcePower),
                    Box::new(Value::SourceToughness),
                )),
                Effect::gain_life(Value::SourcePower),
                Effect::new(ModifyPowerToughnessEffect::pump(
                    ChooseSpec::Source,
                    Value::SourcePower,
                    Until::EndOfTurn,
                )),
            ],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            20 + power + 4 + usable,
            "Signed intermediate power must be retained for the sum, while negative life gain becomes zero"
        );
        actual.push((game.current_power(source), game.current_toughness(source)));
        expected.push((Some(power + usable), Some(4 + usable)));
        assert!(game.stack.is_empty());
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(
            (game.current_power(source), game.current_toughness(source)),
            (Some(4), Some(4))
        );
    }
    assert_eq!(
        actual, expected,
        "CR 107.1b's Chameleon Colossus example: a negative X grants +0/+0, rather than reducing power and toughness"
    );
}

#[test]
fn cr_107_1a_b_rounding_keeps_integer_results_and_negative_characteristic_settings() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Value;
    use ironsmith::effects::SetBasePowerToughnessEffect;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, StackEntry, StaticAbility, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (life, rounded_down, rounded_up) in [
        (-5, -3, -2),
        (-4, -2, -2),
        (-3, -2, -1),
        (-2, -1, -1),
        (-1, -1, 0),
        (0, 0, 0),
        (1, 0, 1),
        (2, 1, 1),
        (3, 1, 2),
        (4, 2, 2),
        (5, 2, 3),
    ] {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let protection =
                CardDefinitionBuilder::new(CardId::new(), "Protection for a negative life total")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::static_ability(StaticAbility::you_cant_lose_game()))
                    .build();
            game.create_object_from_definition(&protection, bob, Zone::Battlefield);
            game.player_mut(bob).unwrap().life = life;
            let creature = counter_creature(&mut game, Zone::Battlefield);
            let value = match mode {
                0 => {
                    Value::HalfRoundedDown(Box::new(Value::LifeTotal(PlayerFilter::Specific(bob))))
                }
                1 => Value::HalfLifeTotalRoundedDown(PlayerFilter::Specific(bob)),
                _ => Value::HalfLifeTotalRoundedUp(PlayerFilter::Specific(bob)),
            };
            game.stack.push(StackEntry::ability(
                creature,
                alice,
                vec![Effect::new(SetBasePowerToughnessEffect::new(
                    ChooseSpec::Source,
                    value,
                    4,
                    Until::EndOfTurn,
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                game.current_power(creature),
                Some(if mode == 2 { rounded_up } else { rounded_down }),
                "CR 107.1a/107.1b: life={life}, rounding mode={mode}; setting a characteristic may retain a negative result"
            );
            assert_eq!(game.current_toughness(creature), Some(4));
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert!(
                game.player(bob).unwrap().is_in_game(),
                "The negative life total is legally maintained by a can't-lose effect"
            );
            assert_eq!(game.player(bob).unwrap().life, life);
            assert!(game.object(creature).is_some());
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert_eq!(
                (
                    game.current_power(creature),
                    game.current_toughness(creature)
                ),
                (Some(3), Some(4))
            );
        }
    }
}

#[test]
fn cr_107_2_undefined_noncreature_vehicle_pt_is_zero_when_used_by_an_effect() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Value;
    use ironsmith::{
        CardId, CardType, ChooseSpec, Effect, PowerToughness, StackEntry, Subtype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for zone in [Zone::Hand, Zone::Battlefield] {
        for creature in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut types = vec![CardType::Artifact];
            if creature {
                types.push(CardType::Creature);
            }
            let definition = CardDefinitionBuilder::new(CardId::new(), "Vehicle number reference")
                .card_types(types)
                .subtypes(vec![Subtype::Vehicle])
                .power_toughness(PowerToughness::fixed(3, 4))
                .build();
            let vehicle = game.create_object_from_definition(&definition, alice, zone);
            let source_definition =
                CardDefinitionBuilder::new(CardId::new(), "Power-plus-toughness reader")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let source =
                game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
            game.stack.push(StackEntry::ability(
                source,
                alice,
                vec![Effect::gain_life(Value::Add(
                    Box::new(Value::PowerOf(Box::new(ChooseSpec::SpecificObject(
                        vehicle,
                    )))),
                    Box::new(Value::ToughnessOf(Box::new(ChooseSpec::SpecificObject(
                        vehicle,
                    )))),
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            actual.push(game.player(alice).unwrap().life);
            assert_eq!(game.object(vehicle).unwrap().zone, zone);
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual,
        vec![27, 27, 20, 27],
        "CR 107.2/208.3: an uncrewed Vehicle on the battlefield has no power or toughness; an effect using those undefined numbers uses zero"
    );
}

#[test]
fn cr_107_1c_any_number_choice_includes_zero_and_large_representable_integers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::NumberContext;
    use ironsmith::effect::ExecutionFact;
    use ironsmith::effects::ChooseNumberEffect;
    use ironsmith::{CardId, CardType, Zone};
    struct ChooseInteger {
        value: u32,
        calls: usize,
    }
    impl DecisionMaker for ChooseInteger {
        fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
            self.calls += 1;
            assert_eq!(ctx.min, 0, "Any number includes zero");
            assert_eq!(
                ctx.authored_max, None,
                "An any-number instruction has no authored upper bound"
            );
            assert!(ctx.max >= self.value);
            self.value
        }
    }
    let alice = PlayerId::from_index(0);
    for value in [0, 1, 7, 1_000_000, u32::MAX] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Any-number choice source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut dm = ChooseInteger { value, calls: 0 };
        let outcome = ChooseNumberEffect::unbounded(PlayerFilter::You)
            .execute(
                &mut game,
                &mut EffectContext::new_default(source, alice).with_decision_maker(&mut dm),
            )
            .unwrap();
        assert_eq!(dm.calls, 1);
        assert_eq!(outcome.as_count(), Some(i64::from(value)));
        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ChosenNumber(value))
        );
    }
}

#[test]
fn cr_107_3g_m_entry_trigger_keeps_cast_x_but_later_trigger_uses_zero() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Value;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, ManaCost, ManaSymbol, StackEntry, TurnAction,
        TurnRunner, TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for x in [0, 3] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Entry and later X reference")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(Value::X)],
            ))
            .with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::gain_life(Value::X)],
            ))
            .build();
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let stable = game.object(spell).unwrap().stable_id;
        game.object_mut(spell).unwrap().x_value = Some(x);
        let mut entry = StackEntry::new(spell, alice);
        entry.x_value = Some(x);
        game.stack.push(entry);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let permanent = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.stable_id == stable)
            })
            .unwrap();
        assert_eq!(
            ObjectSnapshot::from_object(game.object(permanent).unwrap(), &game).mana_value(),
            0
        );
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let entry_life = game.player(alice).unwrap().life;
        assert_eq!(
            entry_life,
            20 + x as i32,
            "CR 107.3m preserves cast X for this entry trigger"
        );
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Upkeep);
        assert!(matches!(
            runner.advance(&mut game, &mut queue).unwrap(),
            TurnAction::RunPriority
        ));
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        actual.push((entry_life, game.player(alice).unwrap().life));
        expected.push((20 + x as i32, 20 + x as i32));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual, expected,
        "CR 107.3g/107.3m: the permanent's X is zero after entry; a later upkeep trigger cannot reuse its historical cast choice"
    );
}

#[test]
fn cr_107_3g_m_uncast_x_permanent_entry_trigger_uses_zero() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Value;
    use ironsmith::effects::MoveToZoneEffect;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, Effect, ManaCost, ManaSymbol, Zone};
    let alice = PlayerId::from_index(0);
    for zone in [Zone::Hand, Zone::Graveyard] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Uncast X entry trigger")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
            .with_ability(Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::gain_life(Value::X)],
            ))
            .build();
        let object = game.create_object_from_definition(&definition, alice, zone);
        assert_eq!(game.object(object).unwrap().x_value, None);
        let source_definition =
            CardDefinitionBuilder::new(CardId::new(), "Put X permanent onto battlefield")
                .card_types(vec![CardType::Artifact])
                .build();
        let source =
            game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
        MoveToZoneEffect::new(ChooseSpec::SpecificObject(object), Zone::Battlefield, false)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            &mut game,
            &mut queue,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        let result = ironsmith::game_loop::resolve_stack_entry(&mut game);
        assert!(
            result.is_ok(),
            "CR 107.3g/107.3m: an uncast X permanent must resolve its entry trigger with X=0, zone={zone:?}, result={result:?}"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_107_3j_gained_trigger_uses_its_own_x_definition_or_zero() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effect::Value;
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, Effect, ManaCost, ManaSymbol, StackEntry, TurnAction,
        TurnRunner, TurnRunnerState, Until, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for cast_x in [0, 3] {
        for defines_x in [true, false] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Recipient with historical cast X")
                    .card_types(vec![CardType::Artifact])
                    .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
                    .build();
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let stable = game.object(spell).unwrap().stable_id;
            game.object_mut(spell).unwrap().x_value = Some(cast_x);
            let mut entry = StackEntry::new(spell, alice);
            entry.x_value = Some(cast_x);
            game.stack.push(entry);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let permanent = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable)
                })
                .unwrap();
            for _ in 0..2 {
                game.create_object_from_definition(&definition, alice, Zone::Hand);
            }
            // A defined-X ability lowers its definition into the numeric expression.
            let amount = if defines_x {
                Value::CardsInHand(PlayerFilter::You)
            } else {
                Value::X
            };
            let granted = Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::gain_life(amount)],
            );
            ApplyContinuousEffect::new(
                EffectTarget::Specific(permanent),
                Modification::AddAbilityGeneric(granted),
                Until::Forever,
            )
            .execute(&mut game, &mut EffectContext::new_default(permanent, alice))
            .unwrap();
            let mut queue = TriggerQueue::new();
            let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::Upkeep);
            assert!(matches!(
                runner.advance(&mut game, &mut queue).unwrap(),
                TurnAction::RunPriority
            ));
            assert_eq!(queue.entries.len(), 1);
            ironsmith::game_loop::put_triggers_on_stack_with_dm(
                &mut game,
                &mut queue,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            actual.push(game.player(alice).unwrap().life);
        }
    }
    assert_eq!(
        actual,
        vec![22, 20, 22, 20],
        "CR 107.3j: a gained ability's own definition controls X; without one X is zero, irrespective of the recipient's old cast value"
    );
}

#[test]
fn cr_107_3n_delayed_trigger_retains_the_creating_spells_chosen_x_after_it_leaves() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::Value;
    use ironsmith::effects::ScheduleDelayedTriggerEffect;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        CardId, CardType, Effect, ManaCost, ManaSymbol, StackEntry, TurnAction, TurnRunner,
        TurnRunnerState, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for caster in [alice, bob] {
        for x in [0, 4] {
            for move_again in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "X spell creating delayed gain")
                        .card_types(vec![CardType::Sorcery])
                        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
                        .with_spell_effect(vec![Effect::new(ScheduleDelayedTriggerEffect::new(
                            Trigger::beginning_of_end_step(PlayerFilter::Any),
                            vec![Effect::gain_life(Value::X)],
                            true,
                            vec![],
                            PlayerFilter::You,
                        ))])
                        .build();
                let spell = game.create_object_from_definition(&definition, caster, Zone::Stack);
                game.object_mut(spell).unwrap().x_value = Some(x);
                let mut entry = StackEntry::new(spell, caster);
                entry.x_value = Some(x);
                game.stack.push(entry);
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert!(game.object(spell).is_none());
                let card = game.player(caster).unwrap().graveyard[0];
                assert_eq!(game.object(card).unwrap().x_value, None);
                if move_again {
                    game.move_object_by_effect(card, Zone::Exile).unwrap();
                }
                let mut queue = TriggerQueue::new();
                let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
                assert!(matches!(
                    runner.advance(&mut game, &mut queue).unwrap(),
                    TurnAction::RunPriority
                ));
                assert_eq!(queue.entries.len(), 1);
                assert_eq!(
                    (
                        queue.entries[0].source,
                        queue.entries[0].controller,
                        queue.entries[0].x_value
                    ),
                    (spell, caster, Some(x))
                );
                ironsmith::game_loop::put_triggers_on_stack_with_dm(
                    &mut game,
                    &mut queue,
                    &mut ironsmith::decision::SelectFirstDecisionMaker,
                )
                .unwrap();
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(caster).unwrap().life, 20 + x as i32);
                let other = if caster == alice { bob } else { alice };
                assert_eq!(game.player(other).unwrap().life, 20);
                assert!(game.stack.is_empty());
            }
        }
    }
}

#[test]
fn cr_107_3a_k_two_mana_ability_activations_choose_x_independently_of_cast_x() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::decisions::context::NumberContext;
    use ironsmith::effect::Value;
    use ironsmith::effects::AddScaledManaEffect;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaCost, ManaSymbol, StackEntry, Zone,
    };
    struct ChooseTwoThenOne {
        calls: usize,
    }
    impl DecisionMaker for ChooseTwoThenOne {
        fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
            let value = [2, 1][self.calls];
            self.calls += 1;
            assert!(ctx.min <= value && value <= ctx.max);
            value
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut ability = Ability::activated(
        TotalCost::from_cost(Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::X]))),
        vec![
            Effect::new(AddScaledManaEffect::new(
                vec![ManaSymbol::Green],
                Value::X,
                PlayerFilter::You,
            )),
            Effect::gain_life(Value::X),
        ],
    );
    let AbilityKind::Activated(ref mut activated) = ability.kind else {
        unreachable!()
    };
    activated.mana_output = Some(vec![]);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Two independent X mana abilities")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
        .with_ability(ability.clone())
        .with_ability(ability)
        .build();
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let stable = game.object(spell).unwrap().stable_id;
    game.object_mut(spell).unwrap().x_value = Some(7);
    let mut entry = StackEntry::new(spell, alice);
    entry.x_value = Some(7);
    game.stack.push(entry);
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let permanent = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable)
        })
        .unwrap();
    game.player_mut(alice).unwrap().mana_pool.colorless = 3;
    let mut dm = ChooseTwoThenOne { calls: 0 };
    for (index, expected_life) in [(0, 22), (1, 23)] {
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game, alice, permanent, index, &mut dm,
        )
        .unwrap();
        assert_eq!(game.player(alice).unwrap().life, expected_life);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            3,
            "The announced amount is paid and the same amount produced"
        );
        let events = game.take_pending_trigger_events();
        let spent = events
            .iter()
            .filter_map(|event| event.downcast::<ironsmith::events::ManaUnitSpentEvent>())
            .count();
        let produced: usize = events
            .iter()
            .filter_map(|event| event.downcast::<ironsmith::events::ManaAddedEvent>())
            .filter(|event| event.source == permanent)
            .map(|event| event.mana.len())
            .sum();
        let chosen = if index == 0 { 2 } else { 1 };
        assert_eq!(
            (spent, produced),
            (chosen, chosen),
            "Each activation pays and produces its own announced X"
        );
        assert!(game.stack.is_empty());
    }
    assert_eq!(dm.calls, 2);
}

#[test]
fn cr_107_3h_pay_referenced_x_mana_cost_uses_stack_x_or_zero_elsewhere() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ManaCost, ManaSymbol, StackEntry, Zone};
    struct PayReference {
        offers: usize,
        payments: usize,
    }
    impl DecisionMaker for PayReference {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            true
        }
        fn decide_mana_payment(
            &mut self,
            _game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.payments += 1;
            assert!(
                ctx.plan.payable,
                "Only the referenced object's X, not the paying effect's X, contributes to the cost"
            );
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (zone, x) in [
        (Zone::Stack, 0),
        (Zone::Stack, 3),
        (Zone::Battlefield, 0),
        (Zone::Battlefield, 3),
        (Zone::Hand, 0),
        (Zone::Graveyard, 0),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Referenced double-X mana cost")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::X,
                ManaSymbol::X,
                ManaSymbol::Green,
            ]))
            .build();
        let object = game.create_object_from_definition(
            &definition,
            bob,
            if zone == Zone::Battlefield {
                Zone::Stack
            } else {
                zone
            },
        );
        let stable = game.object(object).unwrap().stable_id;
        if matches!(zone, Zone::Stack | Zone::Battlefield) {
            game.object_mut(object).unwrap().x_value = Some(x);
            let mut entry = StackEntry::new(object, bob);
            entry.x_value = Some(x);
            game.stack.push(entry);
        }
        let referenced = if zone == Zone::Battlefield {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            game.battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable)
                })
                .unwrap()
        } else {
            object
        };
        let dynamic = ironsmith_core::DynamicManaCost::from_object_mana_cost(
            ChooseSpec::SpecificObject(referenced),
        );
        let parent_definition = CardDefinitionBuilder::new(
            CardId::new(),
            "Different-X spell requesting a referenced cost",
        )
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X]))
        .with_spell_effect(vec![Effect::new(UnlessPaysEffect::new_total_cost(
            vec![Effect::gain_life(5)],
            PlayerFilter::Specific(alice),
            TotalCost::from_cost(Cost::dynamic_mana(dynamic)),
        ))])
        .build();
        let parent = game.create_object_from_definition(&parent_definition, alice, Zone::Stack);
        game.object_mut(parent).unwrap().x_value = Some(9);
        let mut entry = StackEntry::new(parent, alice);
        entry.x_value = Some(9);
        game.stack.push(entry);
        game.player_mut(alice).unwrap().mana_pool.green = 1;
        game.player_mut(alice).unwrap().mana_pool.colorless = 10;
        let mut dm = PayReference {
            offers: 0,
            payments: 0,
        };
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!((dm.offers, dm.payments), (1, 1));
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.colorless,
            if zone == Zone::Stack { 10 - 2 * x } else { 10 },
            "CR 107.3h: referenced zone={zone:?}, referenced historical X={x}"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.stack.len(), usize::from(zone == Zone::Stack));
    }
}

#[test]
fn cr_118_5_6_zero_mana_cost_needs_acknowledgment_but_absent_mana_cost_is_unpayable() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::{CardId, CardType, ChooseSpec, Effect, ManaCost, StackEntry, Zone};
    struct Answer {
        accept: bool,
        offers: usize,
    }
    impl DecisionMaker for Answer {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            self.accept
        }
    }
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for has_zero_cost in [true, false] {
        for accept in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut builder =
                CardDefinitionBuilder::new(CardId::new(), "Zero or absent mana cost reference")
                    .card_types(vec![CardType::Artifact]);
            if has_zero_cost {
                builder = builder.mana_cost(ManaCost::new());
            }
            let referenced =
                game.create_object_from_definition(&builder.build(), alice, Zone::Battlefield);
            let dynamic = ironsmith_core::DynamicManaCost::from_object_mana_cost(
                ChooseSpec::SpecificObject(referenced),
            );
            game.stack.push(StackEntry::ability(
                referenced,
                alice,
                vec![Effect::new(UnlessPaysEffect::new_total_cost(
                    vec![Effect::gain_life(5)],
                    PlayerFilter::Specific(alice),
                    TotalCost::from_cost(Cost::dynamic_mana(dynamic)),
                ))],
            ));
            let mut dm = Answer { accept, offers: 0 };
            let result = ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm);
            assert!(
                result.is_ok(),
                "CR 118.6: inability to pay an absent mana cost selects the consequence instead of aborting resolution: {result:?}"
            );
            assert_eq!(dm.offers, 1);
            assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            assert!(game.stack.is_empty());
            actual.push(game.player(alice).unwrap().life);
            expected.push(if has_zero_cost && accept { 20 } else { 25 });
        }
    }
    assert_eq!(
        actual, expected,
        "CR 118.5/118.6: zero cost is paid only when acknowledged, while a missing mana cost cannot be paid"
    );
}

#[test]
fn cr_107_3j_gained_mana_ability_with_undefined_x_resolves_with_zero() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::cost::TotalCost;
    use ironsmith::effect::Value;
    use ironsmith::effects::{AddScaledManaEffect, ApplyContinuousEffect};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, Until, Zone};
    let alice = PlayerId::from_index(0);
    for defines_x in [true, false] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Recipient without any printed X")
                .card_types(vec![CardType::Artifact])
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..2 {
            game.create_object_from_definition(&definition, alice, Zone::Hand);
        }
        let amount = if defines_x {
            Value::CardsInHand(PlayerFilter::You)
        } else {
            Value::X
        };
        let gained = Ability::mana_with_effects(
            TotalCost::free(),
            vec![Effect::new(AddScaledManaEffect::new(
                vec![ManaSymbol::Green],
                amount,
                PlayerFilter::You,
            ))],
        );
        ApplyContinuousEffect::new(
            EffectTarget::Specific(source),
            Modification::AddAbilityGeneric(gained),
            Until::Forever,
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        let result = ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        assert!(
            result.is_ok(),
            "CR 107.3j: gained mana ability defines X or uses zero, without needing a new X choice; defines X={defines_x}, result={result:?}"
        );
        assert!(game.is_tapped(source));
        assert_eq!(
            game.player(alice).unwrap().mana_pool.green,
            if defines_x { 2 } else { 0 }
        );
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_118_11_stun_replacement_still_pays_an_untap_activation_cost() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, CounterType, Effect, ManaSymbol, Zone,
    };
    let alice = PlayerId::from_index(0);
    for stuns in [0, 1, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut ability = Ability::activated(
            TotalCost::from_cost(Cost::untap()),
            vec![Effect::add_mana(vec![ManaSymbol::Green])],
        );
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.mana_output = Some(vec![]);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Untap-cost mana source")
            .card_types(vec![CardType::Artifact])
            .with_ability(ability)
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.tap(source);
        if stuns > 0 {
            game.object_mut(source)
                .unwrap()
                .add_counters(CounterType::Stun, stuns);
        }
        for attempt in 0..=stuns {
            let result = ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            );
            assert!(
                result.is_ok(),
                "CR 118.11/122.1d: removing a stun counter instead of untapping still pays the cost; initial stuns={stuns}, activation={attempt}, result={result:?}"
            );
            assert_eq!(
                game.counter_count(source, CounterType::Stun),
                stuns.saturating_sub(attempt + 1)
            );
            assert_eq!(game.is_tapped(source), attempt < stuns);
            assert_eq!(game.player(alice).unwrap().mana_pool.green, attempt + 1);
            assert!(game.stack.is_empty());
        }
        assert!(
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker
            )
            .is_err(),
            "An already untapped source cannot pay an untap cost again"
        );
        assert_eq!(game.player(alice).unwrap().mana_pool.green, stuns + 1);
    }
}

#[test]
fn cr_118_3_and_118_12a_unless_composite_payment_is_all_or_nothing() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{Ability, CardId, CardType, Effect, ObjectFilter, StackEntry, Zone};
    struct Willingness {
        accept: bool,
        calls: usize,
    }
    impl DecisionMaker for Willingness {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.calls += 1;
            self.accept
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for life in [1, 2, 3] {
        for initially_tapped in [false, true] {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.player_mut(alice).unwrap().life = life;
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Composite-payment source")
                        .card_types(vec![CardType::Artifact])
                        .with_ability(Ability::static_ability(
                            ironsmith::StaticAbility::you_cant_lose_game(),
                        ))
                        .build();
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                if initially_tapped {
                    game.tap(source);
                }
                let mut filter = ObjectFilter::permanent();
                filter.specific = Some(source);
                let observer_definition =
                    CardDefinitionBuilder::new(CardId::new(), "Payment tap observer")
                        .card_types(vec![CardType::Enchantment])
                        .with_ability(Ability::triggered(
                            Trigger::permanent_becomes_tapped(filter),
                            vec![Effect::gain_life(3)],
                        ))
                        .build();
                let observer = game.create_object_from_definition(
                    &observer_definition,
                    bob,
                    Zone::Battlefield,
                );
                game.stack.push(StackEntry::ability(
                    source,
                    alice,
                    vec![Effect::new(UnlessPaysEffect::new_total_cost(
                        vec![Effect::gain_life(5)],
                        PlayerFilter::Specific(alice),
                        TotalCost::from_costs(vec![Cost::tap(), Cost::life(2)]),
                    ))],
                ));
                let mut dm = Willingness { accept, calls: 0 };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let paid = accept && !initially_tapped && life >= 2;
                assert_eq!(dm.calls, 1);
                assert_eq!(
                    game.is_tapped(source),
                    initially_tapped || paid,
                    "Unaffordable later life component must not leak an earlier tap"
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if paid { life - 2 } else { life + 5 }
                );
                assert_eq!(game.player(bob).unwrap().life, 20);
                let mut queue = TriggerQueue::new();
                ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
                assert!(game.player(alice).unwrap().is_in_game());
                assert_eq!(
                    queue.entries.len(),
                    usize::from(paid),
                    "Only a completed payment may expose the tap trigger; life={life}, initial tap={initially_tapped}, accepted={accept}"
                );
                if paid {
                    assert_eq!(
                        (queue.entries[0].source, queue.entries[0].controller),
                        (observer, bob)
                    );
                    ironsmith::game_loop::put_triggers_on_stack_with_dm(
                        &mut game, &mut queue, &mut dm,
                    )
                    .unwrap();
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                    assert_eq!(game.player(bob).unwrap().life, 23);
                }
                assert!(game.stack.is_empty());
            }
        }
    }
}

#[test]
fn cr_118_11_and_614_1_replacement_can_modify_a_tap_cost_without_undoing_payment() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::events::permanents::matchers::WouldBecomeTappedMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{Ability, CardId, CardType, Effect, ManaSymbol, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for replace_tap in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Replaceable tap-cost mana source")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        if replace_tap {
            let replacement_definition =
                CardDefinitionBuilder::new(CardId::new(), "Source of tap replacement")
                    .card_types(vec![CardType::Enchantment])
                    .build();
            let replacement_source = game.create_object_from_definition(
                &replacement_definition,
                alice,
                Zone::Battlefield,
            );
            let mut filter = ObjectFilter::permanent();
            filter.specific = Some(source);
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    replacement_source,
                    alice,
                    WouldBecomeTappedMatcher::new(filter),
                    ReplacementAction::Instead(vec![Effect::gain_life(2)]),
                ));
            let mut probe = game.clone();
            let provenance = probe
                .provenance_graph_mut()
                .alloc_root_event(ironsmith::events::EventKind::BecomeTapped);
            let event = ironsmith::events::Event::new_with_provenance(
                ironsmith::events::permanents::TapEvent::new(source),
                provenance,
            );
            let result =
                ironsmith::events::processing::process_trait_event(&mut probe, event).unwrap();
            assert!(
                matches!(result, ironsmith::events::processing::TraitEventResult::Replaced { source: id, controller, .. } if id == replacement_source && controller == alice),
                "The registered replacement matches this exact source's would-tap event"
            );
        }
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        actual.push((
            game.is_tapped(source),
            game.player(alice).unwrap().life,
            game.player(alice).unwrap().mana_pool.green,
        ));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![(true, 20, 1), (false, 22, 1)],
        "CR 614.1/118.11: replace the cost's tap event with life gain; the modified cost remains paid and the mana ability resolves"
    );
}

#[test]
fn cr_118_3c_mandatory_mana_payment_does_not_force_activation_of_a_mana_ability() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decisions::context::ManaPaymentContext;
    use ironsmith::effects::PayManaEffect;
    use ironsmith::mana_payment::ManaPaymentResponse;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, ManaCost, ManaSymbol, ObjectId, StackEntry,
        Zone,
    };
    struct RefuseFunding {
        source: ObjectId,
        calls: usize,
    }
    impl DecisionMaker for RefuseFunding {
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            assert!(!game.is_tapped(self.source));
            assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, 0);
            assert!(ctx.plan.payable);
            assert!(!ctx.plan.mana_ability_steps.is_empty());
            ManaPaymentResponse::Cancel
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let definition =
        CardDefinitionBuilder::new(CardId::new(), "Optional funding for a mandatory request")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]))
            .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let parent_definition =
        CardDefinitionBuilder::new(CardId::new(), "Mandatory payment instruction")
            .card_types(vec![CardType::Artifact])
            .build();
    let parent = game.create_object_from_definition(&parent_definition, alice, Zone::Battlefield);
    game.stack.push(StackEntry::ability(
        parent,
        alice,
        vec![
            Effect::new(PayManaEffect::new(
                ManaCost::from_symbols(vec![ManaSymbol::Green]),
                ChooseSpec::Player(PlayerFilter::You),
            )),
            Effect::gain_life(1),
        ],
    ));
    let mut dm = RefuseFunding { source, calls: 0 };
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(dm.calls, 1);
    assert!(!game.is_tapped(source));
    assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
    assert_eq!(
        game.player(alice).unwrap().life,
        21,
        "Resolution continues after the unpayable instruction"
    );
    assert!(game.stack.is_empty());
}

#[test]
fn cr_118_7g_snow_reduction_reduces_generic_and_never_the_snow_requirement() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::LegalAction;
    use ironsmith::static_abilities::{CostReductionManaCost, StaticAbility};
    use ironsmith::{Ability, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, Phase, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut quotes = Vec::new();
    for (base, expected) in [
        (
            vec![ManaSymbol::Generic(2), ManaSymbol::Green],
            vec![ManaSymbol::Generic(1), ManaSymbol::Green],
        ),
        (
            vec![ManaSymbol::Generic(2), ManaSymbol::Snow, ManaSymbol::Green],
            vec![ManaSymbol::Generic(1), ManaSymbol::Snow, ManaSymbol::Green],
        ),
        (
            vec![ManaSymbol::Snow, ManaSymbol::Green],
            vec![ManaSymbol::Snow, ManaSymbol::Green],
        ),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        let base = ManaCost::from_symbols(base);
        let reduction = ManaCost::from_symbols(vec![ManaSymbol::Snow]);
        let canonical = |cost: &ManaCost| {
            let mut pips = cost.pips().to_vec();
            pips.sort_by_key(|pip| format!("{pip:?}"));
            pips
        };
        let choices = base.reduced_by_mana_cost_options(&reduction);
        assert_eq!(choices.len(), 1);
        assert_eq!(
            canonical(&choices[0]),
            canonical(&ManaCost::from_symbols(expected)),
            "Core reduction helper implements snow-as-generic correctly"
        );
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);
        let reducer_definition =
            CardDefinitionBuilder::new(CardId::new(), "Snow-symbol cost reducer")
                .card_types(vec![CardType::Enchantment])
                .with_ability(Ability::static_ability(StaticAbility::new(
                    CostReductionManaCost::new(filter, reduction),
                )))
                .build();
        game.create_object_from_definition(&reducer_definition, alice, Zone::Battlefield);
        let spell_definition =
            CardDefinitionBuilder::new(CardId::new(), "Spell retaining a snow requirement")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(base.clone())
                .build();
        let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.green = 1;
        game.player_mut(alice).unwrap().mana_pool.colorless = 10;
        quotes.push(ironsmith::decision::calculate_effective_mana_cost(
            &game,
            alice,
            game.object(spell).unwrap(),
            &base,
        ));
        let actions = ironsmith::decision::compute_legal_actions(&game, alice).unwrap();
        actual.push(actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)));
    }
    assert_eq!(
        actual,
        vec![true, false, false],
        "CR 118.7g: no snow-produced mana is available; a snow reduction cannot erase the spell's snow pip. Runtime quotes: {quotes:?}"
    );
}

#[test]
fn cr_118_7e_hybrid_reduction_allows_either_half_regardless_of_cost_pip_order() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::LegalAction;
    use ironsmith::static_abilities::{CostReductionManaCost, StaticAbility};
    use ironsmith::{Ability, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, Phase, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for green_first in [true, false] {
        for available_green in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let base = ManaCost::from_symbols(if green_first {
                vec![ManaSymbol::Green, ManaSymbol::Red]
            } else {
                vec![ManaSymbol::Red, ManaSymbol::Green]
            });
            let reduction = ManaCost::from_pips(vec![vec![ManaSymbol::Green, ManaSymbol::Red]]);
            let choices = base.reduced_by_mana_cost_options(&reduction);
            assert_eq!(choices.len(), 2);
            assert!(choices.contains(&ManaCost::from_symbols(vec![ManaSymbol::Green])));
            assert!(choices.contains(&ManaCost::from_symbols(vec![ManaSymbol::Red])));
            let mut filter = ObjectFilter::default();
            filter.cast_by = Some(PlayerFilter::You);
            let reducer_definition =
                CardDefinitionBuilder::new(CardId::new(), "Hybrid-symbol cost reducer")
                    .card_types(vec![CardType::Enchantment])
                    .with_ability(Ability::static_ability(StaticAbility::new(
                        CostReductionManaCost::new(filter, reduction),
                    )))
                    .build();
            game.create_object_from_definition(&reducer_definition, alice, Zone::Battlefield);
            let spell_definition =
                CardDefinitionBuilder::new(CardId::new(), "Two-color spell after hybrid reduction")
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(base)
                    .build();
            let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Hand);
            if available_green {
                game.player_mut(alice).unwrap().mana_pool.green = 1;
            } else {
                game.player_mut(alice).unwrap().mana_pool.red = 1;
            }
            let actions = ironsmith::decision::compute_legal_actions(&game, alice).unwrap();
            actual.push(actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)));
        }
    }
    assert_eq!(
        actual,
        vec![true; 4],
        "CR 118.7e: payer may choose the half that removes the unavailable color; printed pip order cannot select the reduction for them"
    );
}

#[test]
fn cr_118_7a_b_c_d_f_fixed_reductions_preserve_other_symbols_and_spill_to_generic() {
    use ManaSymbol::{Colorless as C, Generic as N, Green as G, Red as R, Snow as S, White as W};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::PaymentReason;
    use ironsmith::static_abilities::{CostReductionManaCost, StaticAbility};
    use ironsmith::{
        Ability, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, Supertype, Zone,
    };
    let alice = PlayerId::from_index(0);
    let cases = vec![
        (
            vec![N(2), G, C, S],
            ManaCost::from_symbols(vec![N(5)]),
            vec![G, C, S],
        ),
        (
            vec![N(2), G, C],
            ManaCost::from_symbols(vec![W]),
            vec![N(1), G, C],
        ),
        (
            vec![N(2), W, G, C],
            ManaCost::from_symbols(vec![W, W]),
            vec![N(1), G, C],
        ),
        (
            vec![N(2), G],
            ManaCost::from_symbols(vec![C]),
            vec![N(1), G],
        ),
        (
            vec![N(2), C, G],
            ManaCost::from_symbols(vec![C, C]),
            vec![N(1), G],
        ),
        (
            vec![N(2), G, R],
            ManaCost::from_pips(vec![vec![G, ManaSymbol::Life(2)]]),
            vec![N(2), R],
        ),
        (
            vec![N(2), R],
            ManaCost::from_pips(vec![vec![G, ManaSymbol::Life(2)]]),
            vec![N(1), R],
        ),
        (vec![G, C], ManaCost::from_symbols(vec![W]), vec![G, C]),
        (vec![G, C], ManaCost::from_symbols(vec![N(5)]), vec![G, C]),
        (
            vec![N(3), W, C, G],
            ManaCost::from_symbols(vec![N(1), W, C]),
            vec![N(2), G],
        ),
        (vec![N(2), G], ManaCost::new(), vec![N(2), G]),
        (vec![], ManaCost::from_symbols(vec![N(9)]), vec![]),
    ];
    let canonical = |cost: &ManaCost| {
        let mut pips = cost.pips().to_vec();
        pips.sort_by_key(|pip| format!("{pip:?}"));
        pips
    };
    for (base_symbols, reduction, expected_symbols) in cases {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let base = ManaCost::from_symbols(base_symbols);
        let expected = ManaCost::from_symbols(expected_symbols);
        let mut filter = ObjectFilter::default();
        filter.cast_by = Some(PlayerFilter::You);
        let reducer_definition =
            CardDefinitionBuilder::new(CardId::new(), "Fixed-symbol cost reducer")
                .card_types(vec![CardType::Enchantment])
                .with_ability(Ability::static_ability(StaticAbility::new(
                    CostReductionManaCost::new(filter, reduction.clone()),
                )))
                .build();
        game.create_object_from_definition(&reducer_definition, alice, Zone::Battlefield);
        let spell_definition =
            CardDefinitionBuilder::new(CardId::new(), "Fixed cost-reduction payment subject")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(base.clone())
                .build();
        let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
        let quote = ironsmith::decision::calculate_effective_mana_cost(
            &game,
            alice,
            game.object(spell).unwrap(),
            &base,
        );
        assert_eq!(
            canonical(&quote),
            canonical(&expected),
            "CR 118.7a-d/f: base={base:?}, reduction={reduction:?}"
        );
        for symbol in [W, ManaSymbol::Blue, ManaSymbol::Black, R, G, C] {
            game.player_mut(alice).unwrap().mana_pool.add(symbol, 10);
        }
        let snow_definition =
            CardDefinitionBuilder::new(CardId::new(), "Snow provenance for a remaining snow pip")
                .card_types(vec![CardType::Artifact])
                .supertypes(vec![Supertype::Snow])
                .with_ability(Ability::mana(TotalCost::free(), vec![G]))
                .build();
        let snow_source =
            game.create_object_from_definition(&snow_definition, alice, Zone::Battlefield);
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            snow_source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        let before = game.player(alice).unwrap().mana_pool.total();
        assert!(
            game.try_pay_mana_cost_with_reason(
                alice,
                Some(spell),
                &quote,
                0,
                PaymentReason::CastSpell
            )
            .unwrap()
        );
        assert_eq!(
            before - game.player(alice).unwrap().mana_pool.total(),
            expected.mana_value()
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "Phyrexian reduction never requires a life payment"
        );
        assert_eq!(
            game.object(spell).unwrap().mana_cost.as_deref(),
            Some(&base),
            "Cost reductions do not change the printed mana cost"
        );
    }
}

#[test]
fn cr_118_11_12_if_you_do_follows_draw_cost_acknowledgment_when_draw_is_skipped() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::MayEffect;
    use ironsmith::replacement::ReplacementEffect;
    use ironsmith::{CardId, CardType, Effect, StackEntry, Zone};
    struct AcceptCost {
        accept: bool,
        offers: usize,
    }
    impl DecisionMaker for AcceptCost {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            self.accept
        }
    }
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for accept in [false, true] {
        for skip_draw in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition = CardDefinitionBuilder::new(CardId::new(), "Optional draw-cost source")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.create_object_from_definition(&definition, alice, Zone::Library);
            if skip_draw {
                game.effect_store
                    .replacement_effects
                    .add_effect(ReplacementEffect::skip_draw(
                        source,
                        alice,
                        PlayerFilter::You,
                    ));
            }
            let cost = MayEffect::new(vec![Effect::draw(1)]).with_pay_as_cost(true);
            game.stack.push(StackEntry::ability(
                source,
                alice,
                vec![
                    Effect::with_id(631, Effect::new(cost)),
                    Effect::if_then_else(
                        EffectId(631),
                        EffectPredicate::Happened,
                        vec![Effect::gain_life(3)],
                        vec![Effect::gain_life(1)],
                    ),
                ],
            ));
            let mut dm = AcceptCost { accept, offers: 0 };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.offers, 1);
            let drawn = usize::from(accept && !skip_draw);
            assert_eq!(game.player(alice).unwrap().hand.len(), drawn);
            assert_eq!(game.player(alice).unwrap().library.len(), 1 - drawn);
            actual.push(game.player(alice).unwrap().life);
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual,
        vec![21, 21, 23, 23],
        "CR 118.11/118.12: choosing and paying a draw cost remains 'if you do' even when replacement skips the physical draw"
    );
}

#[test]
fn cr_118_12b_if_you_search_does_not_require_finding_or_moving_a_card() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::{MayEffect, SearchLibraryEffect};
    use ironsmith::{CardId, CardType, Effect, ObjectFilter, ObjectId, StackEntry, Zone};
    struct SearchChoice {
        accept: bool,
        find: bool,
        card: ObjectId,
        offers: usize,
        searches: usize,
    }
    impl DecisionMaker for SearchChoice {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            self.accept
        }
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.searches += 1;
            assert_eq!((ctx.min, ctx.max), (0, Some(1)));
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == self.card && candidate.legal)
            );
            if self.find { vec![self.card] } else { vec![] }
        }
    }
    let alice = PlayerId::from_index(0);
    for accept in [false, true] {
        for find in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Optional search source and filler")
                    .card_types(vec![CardType::Artifact])
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.create_object_from_definition(&definition, alice, Zone::Library);
            let card = counter_creature(&mut game, Zone::Library);
            let search = SearchLibraryEffect::to_hand(
                ObjectFilter::creature().in_zone(Zone::Library),
                PlayerFilter::You,
                false,
            );
            game.stack.push(StackEntry::ability(
                source,
                alice,
                vec![
                    Effect::with_id(632, Effect::new(MayEffect::new(vec![Effect::new(search)]))),
                    Effect::if_then_else(
                        EffectId(632),
                        EffectPredicate::SearchedLibrary,
                        vec![Effect::gain_life(3)],
                        vec![Effect::gain_life(1)],
                    ),
                ],
            ));
            let mut dm = SearchChoice {
                accept,
                find,
                card,
                offers: 0,
                searches: 0,
            };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.offers, 1);
            assert_eq!(dm.searches, usize::from(accept));
            let found = usize::from(accept && find);
            assert_eq!(game.player(alice).unwrap().hand.len(), found);
            assert_eq!(game.player(alice).unwrap().library.len(), 2 - found);
            assert_eq!(
                game.player(alice).unwrap().life,
                if accept { 23 } else { 21 },
                "CR 118.12b: choosing the search is sufficient whether a stated-quality card is found or not"
            );
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_118_12_mandatory_source_sacrifice_if_do_uses_payment_not_graveyard_arrival() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::events::zones::matchers::WouldChangeZoneMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{CardId, CardType, Effect, ObjectFilter, StackEntry, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for mode in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "Mandatory sacrifice-if-do source")
                .card_types(vec![CardType::Enchantment])
                .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        if mode == 1 {
            let replacement_source =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    replacement_source,
                    alice,
                    WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(source),
                        Some(Zone::Battlefield),
                        Some(Zone::Graveyard),
                    ),
                    ReplacementAction::ChangeDestination(Zone::Exile),
                ));
        }
        game.stack.push(StackEntry::ability(
            source,
            alice,
            Effect::do_if_do(633, Effect::sacrifice_source(), vec![Effect::gain_life(3)]),
        ));
        if mode == 2 {
            game.move_object_by_effect(source, Zone::Exile).unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let expected_zone = if mode == 0 {
            Zone::Graveyard
        } else {
            Zone::Exile
        };
        let cards = if expected_zone == Zone::Graveyard {
            &game.player(alice).unwrap().graveyard
        } else {
            &game.exile
        };
        assert!(cards.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable)
        }));
        actual.push(game.player(alice).unwrap().life);
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![23, 23, 20],
        "CR 118.12: source sacrificed to exile pays the cost, but an already departed source cannot start paying it"
    );
}

#[test]
fn cr_118_12_optional_entry_cost_stays_paid_when_replacement_changes_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::{MayEffect, MoveToZoneEffect, ReturnToHandEffect};
    use ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{
        CardId, CardType, ChooseSpec, Effect, ObjectFilter, PowerToughness, StackEntry, Zone,
    };
    struct ChooseEntry {
        accept: bool,
        offers: usize,
    }
    impl DecisionMaker for ChooseEntry {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            self.accept
        }
    }
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for accept in [false, true] {
        for redirect in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source_definition =
                CardDefinitionBuilder::new(CardId::new(), "Source returned after an entry cost")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(2, 2))
                    .build();
            let source =
                game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
            let source_stable = game.object(source).unwrap().stable_id;
            let card = counter_creature(&mut game, Zone::Hand);
            let card_stable = game.object(card).unwrap().stable_id;
            if redirect {
                let replacement_source =
                    game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
                game.effect_store
                    .replacement_effects
                    .add_effect(ReplacementEffect::with_matcher(
                        replacement_source,
                        bob,
                        WouldEnterBattlefieldMatcher::new(ObjectFilter::creature())
                            .with_stable_id(card_stable),
                        ReplacementAction::EnterUnderControl(bob),
                    ));
            }
            let mut card_filter = ObjectFilter::creature()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::You);
            card_filter.specific = Some(card);
            let cost = MayEffect::new(vec![Effect::new(
                MoveToZoneEffect::new(ChooseSpec::Object(card_filter), Zone::Battlefield, false)
                    .under_you_control(),
            )])
            .with_pay_as_cost(true);
            game.stack.push(StackEntry::ability(
                source,
                alice,
                vec![
                    Effect::with_id(634, Effect::new(cost)),
                    Effect::if_then(
                        EffectId(634),
                        EffectPredicate::Happened,
                        vec![Effect::new(ReturnToHandEffect::with_spec(
                            ChooseSpec::Source,
                        ))],
                    ),
                ],
            ));
            let mut dm = ChooseEntry { accept, offers: 0 };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(dm.offers, 1);
            let in_hand = |stable| {
                game.player(alice).unwrap().hand.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable)
                })
            };
            assert_eq!(
                in_hand(source_stable),
                accept,
                "CR 118.12: accepted entry cost returns source even when entry controller is replaced"
            );
            assert_eq!(in_hand(card_stable), !accept);
            if accept {
                let entered = game
                    .battlefield
                    .iter()
                    .copied()
                    .find(|id| {
                        game.object(*id)
                            .is_some_and(|object| object.stable_id == card_stable)
                    })
                    .unwrap();
                assert_eq!(
                    game.current_controller(entered),
                    Some(if redirect { bob } else { alice })
                );
            } else {
                assert!(game.object(source).is_some());
            }
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_601_2h_player_can_tap_before_sacrificing_source_regardless_of_cost_storage_order() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{Ability, AbilityKind, CardId, CardType, Effect, ManaSymbol, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for sacrifice_first in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let costs = if sacrifice_first {
            vec![Cost::sacrifice_self(), Cost::tap()]
        } else {
            vec![Cost::tap(), Cost::sacrifice_self()]
        };
        let mut ability = Ability::activated(
            TotalCost::from_costs(costs),
            vec![Effect::add_mana(vec![ManaSymbol::Green])],
        );
        let AbilityKind::Activated(ref mut activated) = ability.kind else {
            unreachable!()
        };
        activated.mana_output = Some(vec![]);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Tap-and-sacrifice mana source")
            .card_types(vec![CardType::Artifact])
            .with_ability(ability)
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let result = ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            alice,
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        );
        let in_graveyard = game.player(alice).unwrap().graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable)
        });
        actual.push((
            result.is_ok(),
            game.player(alice).unwrap().mana_pool.green,
            in_graveyard,
        ));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual,
        vec![(true, 1, true), (true, 1, true)],
        "CR 602.2b/601.2h: the player can pay the tap cost before sacrificing the same permanent in either representation"
    );
}

#[test]
fn cr_118_10_one_sacrifice_cannot_pay_two_abilities_or_be_reused_from_resolution() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, Effect, ManaSymbol, ObjectFilter, StackEntry, Zone,
    };
    let alice = PlayerId::from_index(0);
    for resources in [0, 1, 2] {
        for sacrifice_by_effect in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut ability = Ability::activated(
                TotalCost::from_cost(Cost::sacrifice(ObjectFilter::creature().you_control())),
                vec![Effect::add_mana(vec![ManaSymbol::Green])],
            );
            let AbilityKind::Activated(ref mut activated) = ability.kind else {
                unreachable!()
            };
            activated.mana_output = Some(vec![]);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Creature-sacrifice mana source")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(ability)
                    .build();
            let sources = [
                game.create_object_from_definition(&definition, alice, Zone::Battlefield),
                game.create_object_from_definition(&definition, alice, Zone::Battlefield),
            ];
            for _ in 0..resources {
                counter_creature(&mut game, Zone::Battlefield);
            }
            if sacrifice_by_effect {
                game.stack.push(StackEntry::ability(
                    sources[0],
                    alice,
                    vec![Effect::sacrifice(ObjectFilter::creature().you_control(), 1)],
                ));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.total(),
                    0,
                    "Sacrificing during resolution does not activate either source"
                );
            }
            let available = resources - usize::from(sacrifice_by_effect && resources > 0);
            for (index, source) in sources.into_iter().enumerate() {
                let result = ironsmith::special_actions::perform_activate_mana_ability(
                    &mut game,
                    alice,
                    source,
                    0,
                    &mut ironsmith::decision::SelectFirstDecisionMaker,
                );
                assert_eq!(
                    result.is_ok(),
                    index < available,
                    "CR 118.10: resources={resources}, sacrificed by effect={sacrifice_by_effect}, activation={index}, result={result:?}"
                );
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.green as usize,
                    available.min(index + 1)
                );
                assert!(game.stack.is_empty());
            }
            assert_eq!(game.player(alice).unwrap().graveyard.len(), resources);
        }
    }
}

#[test]
fn cr_118_3_resolution_cost_cannot_sacrifice_one_permanent_twice() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::{CardId, CardType, Effect, ObjectFilter, StackEntry, Zone};
    struct Accept;
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
    }
    let alice = PlayerId::from_index(0);
    for resources in [0, 1, 2] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Double sacrifice offer")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for _ in 0..resources {
            counter_creature(&mut game, Zone::Battlefield);
        }
        let cost = TotalCost::from_costs(vec![
            Cost::sacrifice(ObjectFilter::creature().you_control()),
            Cost::sacrifice(ObjectFilter::creature().you_control()),
        ]);
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![Effect::new(UnlessPaysEffect::new_total_cost(
                vec![Effect::gain_life(5)],
                PlayerFilter::You,
                cost,
            ))],
        ));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap();
        let paid = resources == 2;
        assert_eq!(
            game.player(alice).unwrap().life,
            if paid { 20 } else { 25 },
            "CR 118.3: both sacrifice components require distinct resources; resources={resources}"
        );
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            if paid { 2 } else { 0 },
            "An unaffordable composite cost must not partially sacrifice a creature"
        );
        assert!(game.stack.is_empty());
    }
}

#[test]
fn cr_118_12_and_121_3_draw_cost_acknowledgment_survives_an_empty_library() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effect::{EffectId, EffectPredicate};
    use ironsmith::effects::MayEffect;
    use ironsmith::{Ability, CardId, CardType, Effect, StackEntry, Zone};
    struct Accept;
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
    }
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    for optional in [false, true] {
        for empty in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Empty-library draw cost source")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::static_ability(
                        ironsmith::StaticAbility::you_cant_lose_game(),
                    ))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if !empty {
                counter_creature(&mut game, Zone::Library);
            }
            let action = if optional {
                Effect::new(MayEffect::new(vec![Effect::draw(1)]).with_pay_as_cost(true))
            } else {
                Effect::draw(1)
            };
            game.stack.push(StackEntry::ability(
                source,
                alice,
                vec![
                    Effect::with_id(635, action),
                    Effect::if_then_else(
                        EffectId(635),
                        EffectPredicate::Happened,
                        vec![Effect::gain_life(3)],
                        vec![Effect::gain_life(1)],
                    ),
                ],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap();
            assert_eq!(game.player(alice).unwrap().hand.len(), usize::from(!empty));
            assert_eq!(
                game.player(alice)
                    .unwrap()
                    .attempted_draw_from_empty_library,
                empty
            );
            actual.push(game.player(alice).unwrap().life);
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual,
        vec![23; 4],
        "CR 118.12/121.3: starting the mandatory draw cost or accepting its optional offer pays it even if the library is empty"
    );
}

fn audit_finish_cost_rule_cast(
    game: &mut GameState,
    action: ironsmith::decision::LegalAction,
) -> ironsmith::ObjectId {
    audit_finish_cost_rule_cast_with(
        game,
        action,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    )
}

fn audit_finish_cost_rule_cast_with<D: DecisionMaker>(
    game: &mut GameState,
    action: ironsmith::decision::LegalAction,
    dm: &mut D,
) -> ironsmith::ObjectId {
    use ironsmith::decision::GameProgress;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::triggers::TriggerQueue;
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .expect("An affordable cost-rule cast must start");
    for _ in 0..32 {
        if let Some(entry) = game.stack.last() {
            assert!(state.pending_cast.is_none());
            return entry.object_id;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("Affordable cost-rule cast stopped before going on stack: {progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            game, &mut queue, &mut state, &context, dm,
        )
        .expect("Affordable cost-rule cast payment must finish");
    }
    panic!("Cost-rule cast did not finish");
}

#[test]
fn cr_118_6a_8d_9c_9d_alternative_and_additional_prices_preserve_printed_mana_cost() {
    use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::effect::Value;
    use ironsmith::game_state::Phase;
    use ironsmith::static_abilities::{CostIncrease, CostReduction, StaticAbility};
    use ironsmith::{
        Ability, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, ObjectSnapshot, Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for printed in [None, Some(0), Some(5)] {
        for alternative in [None, Some(0), Some(2)] {
            for additional in [false, true] {
                for adjustment in [-2, 0, 2] {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.priority_player = Some(alice);
                    game.turn.phase = Phase::FirstMain;
                    game.turn.step = None;
                    if adjustment != 0 {
                        let mut filter = ObjectFilter::default();
                        filter.cast_by = Some(PlayerFilter::You);
                        let modifier = if adjustment < 0 {
                            StaticAbility::new(CostReduction::new(
                                filter,
                                Value::Fixed(-adjustment),
                            ))
                        } else {
                            StaticAbility::new(CostIncrease::new(filter, Value::Fixed(adjustment)))
                        };
                        let definition =
                            CardDefinitionBuilder::new(CardId::new(), "Cost-rule price modifier")
                                .card_types(vec![CardType::Enchantment])
                                .with_ability(Ability::static_ability(modifier))
                                .build();
                        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    }
                    let mut builder = CardDefinitionBuilder::new(
                        CardId::new(),
                        "Alternative and additional price subject",
                    )
                    .card_types(vec![CardType::Sorcery]);
                    if let Some(generic) = printed {
                        builder = builder.mana_cost(ManaCost::new().add_generic(generic));
                    }
                    if let Some(generic) = alternative {
                        builder =
                            builder.alternative_cast(AlternativeCastingMethod::alternative_cost(
                                "Independent alternative price",
                                Some(ManaCost::new().add_generic(generic)),
                                vec![],
                            ));
                    }
                    if additional {
                        builder = builder.additional_cost(TotalCost::from_costs(vec![
                            Cost::mana(ManaCost::from_symbols(vec![
                                ManaSymbol::Generic(1),
                                ManaSymbol::Green,
                            ])),
                            Cost::life(2),
                        ]));
                    }
                    let spell =
                        game.create_object_from_definition(&builder.build(), alice, Zone::Hand);
                    game.player_mut(alice).unwrap().mana_pool.colorless = 20;
                    game.player_mut(alice).unwrap().mana_pool.green = u32::from(additional);
                    let method = if alternative.is_some() {
                        CastingMethod::Alternative(0)
                    } else {
                        CastingMethod::Normal
                    };
                    let action = compute_legal_actions(&game, alice)
                        .unwrap()
                        .into_iter()
                        .find(|action| {
                            matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. }
                            if *spell_id == spell && *casting_method == method)
                        });
                    if printed.is_none() && alternative.is_none() {
                        assert!(
                            action.is_none(),
                            "An absent mana cost cannot become payable by addition or reduction alone"
                        );
                        assert!(game.stack.is_empty());
                        continue;
                    }
                    let stack = audit_finish_cost_rule_cast(
                        &mut game,
                        action.expect("Payable native casting route"),
                    );
                    let object = game.object(stack).unwrap();
                    assert_eq!(
                        object.mana_cost.as_ref().map(|cost| cost.mana_value()),
                        printed,
                        "CR 118.8d/118.9c: printed={printed:?}, alternative={alternative:?}, additional={additional}, adjustment={adjustment}"
                    );
                    assert_eq!(
                        ObjectSnapshot::from_object(object, &game).mana_value(),
                        printed.unwrap_or(0)
                    );
                    let generic = (alternative.or(printed).unwrap() as i32
                        + i32::from(additional)
                        + adjustment)
                        .max(0) as u32;
                    actual.push((
                        game.player(alice).unwrap().mana_pool.colorless,
                        game.player(alice).unwrap().mana_pool.green,
                        game.player(alice).unwrap().life,
                    ));
                    expected.push((20 - generic, 0, if additional { 18 } else { 20 }));
                }
            }
        }
    }
    assert_eq!(
        actual, expected,
        "CR 118.6a/118.9d: an alternative can replace an absent mana cost; additional mana, increases and reductions all apply to the chosen price"
    );
}

#[test]
fn cr_118_8a_8b_multiple_optional_additional_costs_are_independently_announced_and_paid() {
    use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::{OptionalCost, TotalCost};
    use ironsmith::costs::Cost;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::effect::Value;
    use ironsmith::game_state::Phase;
    use ironsmith::static_abilities::{CostReduction, StaticAbility};
    use ironsmith::{Ability, CardId, CardType, ManaCost, ObjectFilter, ObjectSnapshot, Zone};
    struct ChooseCosts {
        mask: usize,
        offers: usize,
    }
    impl DecisionMaker for ChooseCosts {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs for ") {
                self.offers += 1;
                assert_eq!(ctx.min, 0);
                assert_eq!(ctx.options.len(), 2);
                return ctx
                    .options
                    .iter()
                    .filter(|option| self.mask & (1 << option.index) != 0)
                    .map(|option| {
                        assert!(option.legal);
                        option.index
                    })
                    .collect();
            }
            if ctx.min == 0 {
                return vec![];
            }
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .take(ctx.min)
                .map(|option| option.index)
                .collect()
        }
    }
    let alice = PlayerId::from_index(0);
    for alternative in [false, true] {
        for reduction in [0, 2] {
            for mask in 0..4 {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                if reduction > 0 {
                    let mut filter = ObjectFilter::default();
                    filter.cast_by = Some(PlayerFilter::You);
                    let definition =
                        CardDefinitionBuilder::new(CardId::new(), "Optional-price reducer")
                            .card_types(vec![CardType::Enchantment])
                            .with_ability(Ability::static_ability(StaticAbility::new(
                                CostReduction::new(filter, Value::Fixed(reduction)),
                            )))
                            .build();
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                }
                let mut builder = CardDefinitionBuilder::new(
                    CardId::new(),
                    "Independent optional additional costs",
                )
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::new().add_generic(5))
                .optional_cost(OptionalCost::custom(
                    "Optional mana",
                    TotalCost::mana(ManaCost::new().add_generic(3)),
                ))
                .optional_cost(OptionalCost::custom(
                    "Optional life",
                    TotalCost::from_cost(Cost::life(2)),
                ));
                if alternative {
                    builder = builder.alternative_cast(AlternativeCastingMethod::alternative_cost(
                        "Free alternative",
                        Some(ManaCost::new()),
                        vec![],
                    ));
                }
                let spell = game.create_object_from_definition(&builder.build(), alice, Zone::Hand);
                game.player_mut(alice).unwrap().mana_pool.colorless = 20;
                let method = if alternative {
                    CastingMethod::Alternative(0)
                } else {
                    CastingMethod::Normal
                };
                let action = compute_legal_actions(&game, alice)
                    .unwrap()
                    .into_iter()
                    .find(|action| {
                        matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. }
                        if *spell_id == spell && *casting_method == method)
                    })
                    .unwrap();
                let mut dm = ChooseCosts { mask, offers: 0 };
                let stack = audit_finish_cost_rule_cast_with(&mut game, action, &mut dm);
                assert_eq!(dm.offers, 1);
                let object = game.object(stack).unwrap();
                for index in 0..2 {
                    assert_eq!(
                        object.optional_costs_paid.was_paid(index),
                        mask & (1 << index) != 0
                    );
                    assert_eq!(
                        game.stack
                            .last()
                            .unwrap()
                            .optional_costs_paid
                            .was_paid(index),
                        mask & (1 << index) != 0
                    );
                }
                assert_eq!(ObjectSnapshot::from_object(object, &game).mana_value(), 5);
                let generic = ((if alternative { 0 } else { 5 })
                    + if mask & 1 != 0 { 3 } else { 0 }
                    - reduction)
                    .max(0) as u32;
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.colorless,
                    20 - generic,
                    "CR 118.8a/118.8b/118.9d: alternative={alternative}, reduction={reduction}, subset={mask}"
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if mask & 2 != 0 { 18 } else { 20 }
                );
            }
        }
    }
}

#[test]
fn cr_118_13b_resolution_phyrexian_cost_allows_each_pip_to_use_mana_or_life() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effects::UnlessPaysEffect;
    use ironsmith::mana_payment::{ManaPaymentResponse, ManaPipId};
    use ironsmith::{CardId, CardType, Effect, ManaCost, ManaSymbol, StackEntry, Zone};
    struct ChoosePayment {
        accept: bool,
        mask: u32,
        pips: u32,
        offers: usize,
        payments: usize,
    }
    impl DecisionMaker for ChoosePayment {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.offers += 1;
            self.accept
        }
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.payments += 1;
            assert_eq!(
                game.player(ctx.player).unwrap().life,
                20,
                "Choose how each Phyrexian pip is paid before any life is spent"
            );
            assert_eq!(
                game.player(ctx.player).unwrap().mana_pool.green,
                self.pips,
                "Replanning must not spend a prefix payment"
            );
            let desired = (0..self.pips)
                .filter(|index| self.mask & (1 << index) != 0)
                .map(ManaPipId)
                .collect::<Vec<_>>();
            if ctx.request.preferences.required_life_pips != desired {
                let mut preferences = ctx.request.preferences.clone();
                preferences.required_life_pips = desired;
                return ManaPaymentResponse::Replan { preferences };
            }
            assert!(ctx.plan.payable);
            assert_eq!(ctx.plan.life_to_pay, 2 * self.mask.count_ones());
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }
    let alice = PlayerId::from_index(0);
    for pips in [1, 2] {
        for mask in 0..(1 << pips) {
            for accept in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Resolving Phyrexian payment")
                        .card_types(vec![CardType::Artifact])
                        .build();
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                game.player_mut(alice).unwrap().mana_pool.green = pips;
                let cost = ManaCost::from_pips(vec![
                    vec![ManaSymbol::Green, ManaSymbol::Life(2)];
                    pips as usize
                ]);
                game.stack.push(StackEntry::ability(
                    source,
                    alice,
                    vec![Effect::new(UnlessPaysEffect::new_total_cost(
                        vec![Effect::gain_life(5)],
                        PlayerFilter::You,
                        TotalCost::from_cost(Cost::mana(cost)),
                    ))],
                ));
                let mut dm = ChoosePayment {
                    accept,
                    mask,
                    pips,
                    offers: 0,
                    payments: 0,
                };
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.offers, 1);
                assert_eq!(
                    dm.payments,
                    if !accept {
                        0
                    } else if mask == 0 {
                        1
                    } else {
                        2
                    }
                );
                let life_pips = mask.count_ones();
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if accept {
                        20 - 2 * life_pips as i32
                    } else {
                        25
                    }
                );
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.green,
                    if accept { life_pips } else { pips }
                );
                assert!(game.stack.is_empty());
            }
        }
    }
}

#[test]
fn cr_118_14_and_609_4b_any_type_changes_payment_but_not_actual_mana_or_snow() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::effect::ManaSpendPermission;
    use ironsmith::effects::RegisterManaSpendPermissionEffect;
    use ironsmith::game_state::Phase;
    use ironsmith::{Ability, CardId, CardType, ManaCost, ManaSymbol, Supertype, Until, Zone};
    use ironsmith_core::value_model::ManaSpendMode;
    let alice = PlayerId::from_index(0);
    let costs = [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
        ManaSymbol::Snow,
    ];
    for mode in [
        ManaSpendMode::Normal,
        ManaSpendMode::AnyColor,
        ManaSpendMode::AnyType,
    ] {
        for actual_symbol in [ManaSymbol::Green, ManaSymbol::Colorless] {
            for snow in [false, true] {
                for cost_symbol in costs {
                    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                    game.turn.active_player = alice;
                    game.turn.priority_player = Some(alice);
                    game.turn.phase = Phase::FirstMain;
                    game.turn.step = None;
                    let mut builder = CardDefinitionBuilder::new(
                        CardId::new(),
                        "Actual mana production for as-though payment",
                    )
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::mana(TotalCost::free(), vec![actual_symbol]));
                    if snow {
                        builder = builder.supertypes(vec![Supertype::Snow]);
                    }
                    let producer = game.create_object_from_definition(
                        &builder.build(),
                        alice,
                        Zone::Battlefield,
                    );
                    ironsmith::special_actions::perform_activate_mana_ability(
                        &mut game,
                        alice,
                        producer,
                        0,
                        &mut ironsmith::decision::SelectFirstDecisionMaker,
                    )
                    .unwrap();
                    if mode != ManaSpendMode::Normal {
                        let mut permission = ManaSpendPermission::any_color(PlayerFilter::You);
                        permission.mode = mode;
                        RegisterManaSpendPermissionEffect {
                            permission,
                            until: Until::EndOfTurn,
                            display: "Audit mana spending permission".into(),
                        }
                        .execute(&mut game, &mut EffectContext::new_default(producer, alice))
                        .unwrap();
                    }
                    let cost = ManaCost::from_symbols(vec![cost_symbol]);
                    let definition = CardDefinitionBuilder::new(
                        CardId::new(),
                        "Mana payment conversion subject",
                    )
                    .card_types(vec![CardType::Sorcery])
                    .mana_cost(cost.clone())
                    .build();
                    let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
                    let expected = match cost_symbol {
                        ManaSymbol::Snow => snow,
                        ManaSymbol::Colorless => {
                            actual_symbol == ManaSymbol::Colorless || mode == ManaSpendMode::AnyType
                        }
                        _ => actual_symbol == cost_symbol || mode != ManaSpendMode::Normal,
                    };
                    let action = compute_legal_actions(&game, alice).unwrap().into_iter().find(|action|
                        matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. } if *spell_id == spell));
                    assert_eq!(
                        action.is_some(),
                        expected,
                        "CR 118.14: mode={mode:?}, actual={actual_symbol:?}, source snow={snow}, required={cost_symbol:?}"
                    );
                    if let Some(action) = action {
                        let stack = audit_finish_cost_rule_cast(&mut game, action);
                        let object = game.object(stack).unwrap();
                        assert_eq!(object.mana_cost.as_ref().unwrap().pips(), cost.pips());
                        assert_eq!(object.mana_spent_to_cast.total(), 1);
                        assert_eq!(
                            object.mana_spent_to_cast.green,
                            u32::from(actual_symbol == ManaSymbol::Green)
                        );
                        assert_eq!(
                            object.mana_spent_to_cast.colorless,
                            u32::from(actual_symbol == ManaSymbol::Colorless)
                        );
                        assert_eq!(
                            object.snow_mana_spent_to_cast.total(),
                            u32::from(snow),
                            "CR 609.4b: conversion neither creates nor removes snow provenance"
                        );
                        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
                    } else {
                        assert!(game.stack.is_empty());
                        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
                    }
                }
            }
        }
    }
}

#[test]
fn cr_118_14_cast_permission_conversion_does_not_leak_to_other_routes_or_resolution() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::{Cost, PaymentReason};
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::effects::{GrantPlayTaggedDuration, GrantPlayTaggedEffect, UnlessPaysEffect};
    use ironsmith::game_state::Phase;
    use ironsmith::{CardId, CardType, Effect, ManaCost, ManaSymbol, ObjectSnapshot, Zone};
    use ironsmith_core::value_model::ManaSpendMode;
    struct Accept;
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            true
        }
    }
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let red = ManaCost::from_symbols(vec![ManaSymbol::Red]);
    let definition = CardDefinitionBuilder::new(
        CardId::new(),
        "Exile spell with a separate resolution payment",
    )
    .card_types(vec![CardType::Sorcery])
    .mana_cost(red.clone())
    .with_spell_effect(vec![Effect::new(UnlessPaysEffect::new_total_cost(
        vec![Effect::gain_life(5)],
        PlayerFilter::You,
        TotalCost::from_cost(Cost::mana(red.clone())),
    ))])
    .build();
    let spell = game.create_object_from_definition(&definition, alice, Zone::Exile);
    let unrelated = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source_definition =
        CardDefinitionBuilder::new(CardId::new(), "Exile casting permission source")
            .card_types(vec![CardType::Artifact])
            .build();
    let any_type_source =
        game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
    let ordinary_source =
        game.create_object_from_definition(&source_definition, alice, Zone::Battlefield);
    for (source, mode) in [
        (any_type_source, ManaSpendMode::AnyType),
        (ordinary_source, ManaSpendMode::Normal),
    ] {
        let mut ctx = EffectContext::new_default(source, alice).with_tagged_objects(
            std::collections::HashMap::from([(
                "permitted_card".into(),
                vec![ObjectSnapshot::from_object(
                    game.object(spell).unwrap(),
                    &game,
                )],
            )]),
        );
        let mut grant = GrantPlayTaggedEffect::new(
            "permitted_card",
            PlayerFilter::You,
            GrantPlayTaggedDuration::UntilEndOfTurn,
            false,
            mode,
        );
        grant.permission_bound_mana = mode != ManaSpendMode::Normal;
        grant.execute(&mut game, &mut ctx).unwrap();
    }
    game.player_mut(alice).unwrap().mana_pool.blue = 2;
    let actions = compute_legal_actions(&game, alice).unwrap();
    assert!(!actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == unrelated)),
        "CR 118.14: one exiled card's casting permission does not convert mana for another card");
    let routes = actions.into_iter().filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).collect::<Vec<_>>();
    assert!(!routes.iter().any(|action| matches!(action, LegalAction::CastSpell { casting_method, .. }
        if matches!(casting_method.without_exact_permission(), CastingMethod::PlayFrom { source, .. } if *source == ordinary_source))),
        "CR 118.14: the competing ordinary permission cannot borrow conversion from another permission");
    let action = routes.into_iter().find(|action| matches!(action, LegalAction::CastSpell { casting_method, .. }
        if matches!(casting_method.without_exact_permission(), CastingMethod::PlayFrom { source, .. } if *source == any_type_source)))
        .expect("The any-type permission can spend blue for the red casting cost");
    let stack = audit_finish_cost_rule_cast(&mut game, action);
    assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.blue, 1);
    assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    assert!(
        !game.can_pay_mana_cost_with_reason(alice, Some(stack), &red, 0, PaymentReason::Effect),
        "Casting-only permission cannot convert the same spell's separate resolving cost"
    );
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 25);
    assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
    assert!(game.stack.is_empty());
}

#[test]
fn cr_118_14_as_though_conversion_preserves_a_mana_units_usage_restriction() {
    use ironsmith::ability::ManaUsageRestriction;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::effect::ManaSpendPermission;
    use ironsmith::effects::RegisterManaSpendPermissionEffect;
    use ironsmith::game_state::Phase;
    use ironsmith::{
        Ability, AbilityKind, CardId, CardType, ManaCost, ManaSymbol, ObjectFilter, PowerToughness,
        Until, Zone,
    };
    use ironsmith_core::value_model::ManaSpendMode;
    let alice = PlayerId::from_index(0);
    for mode in [ManaSpendMode::AnyColor, ManaSpendMode::AnyType] {
        for creature in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let mut ability = Ability::mana(TotalCost::free(), vec![ManaSymbol::Blue]);
            let AbilityKind::Activated(ref mut activated) = ability.kind else {
                unreachable!()
            };
            activated.mana_usage_restrictions = vec![ManaUsageRestriction::CastSpellMatching {
                filter: ObjectFilter::creature(),
                restrict_to_matching_spell: true,
                grant_uncounterable: false,
                enters_with_counters: vec![],
                granted_abilities: vec![],
            }];
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Restricted blue mana source")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(ability)
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game,
                alice,
                source,
                0,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let mut permission = ManaSpendPermission::any_color(PlayerFilter::You);
            permission.mode = mode;
            RegisterManaSpendPermissionEffect {
                permission,
                until: Until::EndOfTurn,
                display: "Convert restricted mana".into(),
            }
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
            let mut builder =
                CardDefinitionBuilder::new(CardId::new(), "Restricted converted payment subject")
                    .card_types(vec![if creature {
                        CardType::Creature
                    } else {
                        CardType::Sorcery
                    }])
                    .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]));
            if creature {
                builder = builder.power_toughness(PowerToughness::fixed(2, 2));
            }
            let spell = game.create_object_from_definition(&builder.build(), alice, Zone::Hand);
            let action = compute_legal_actions(&game, alice).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
            assert_eq!(
                action.is_some(),
                creature,
                "CR 118.14/609.4b: converting blue to pay red does not erase creature-only usage; mode={mode:?}"
            );
            if let Some(action) = action {
                let stack = audit_finish_cost_rule_cast(&mut game, action);
                assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.blue, 1);
                assert_eq!(game.object(stack).unwrap().mana_spent_to_cast.red, 0);
                assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
            } else {
                assert_eq!(game.player(alice).unwrap().mana_pool.blue, 1);
            }
        }
    }
}

#[test]
fn cr_118_13a_cast_hybrid_and_phyrexian_halves_are_announced_before_spending() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_state::Phase;
    use ironsmith::{CardId, CardType, ManaCost, ManaSymbol, Zone};
    struct Announce {
        mask: usize,
        seen: usize,
        pips: u32,
        pool_before: ironsmith::player::ManaPool,
    }
    impl DecisionMaker for Announce {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose how to pay pip ") {
                assert_eq!(ctx.options.len(), 2);
                assert_eq!(game.player(ctx.player).unwrap().life, 20);
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool,
                    self.pool_before,
                    "Every mana type remains unspent until all announcements finish"
                );
                assert_eq!(
                    game.player(ctx.player).unwrap().mana_pool.green,
                    self.pips,
                    "All pip announcements precede the first mana payment"
                );
                let choice = usize::from(self.mask & (1 << self.seen) != 0);
                self.seen += 1;
                return vec![ctx.options[choice].index];
            }
            if ctx.min == 0 {
                return vec![];
            }
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .take(ctx.min)
                .map(|option| option.index)
                .collect()
        }
    }
    let alice = PlayerId::from_index(0);
    for other in [ManaSymbol::Red, ManaSymbol::Generic(2), ManaSymbol::Life(2)] {
        for pips in [1, 2] {
            for mask in 0..(1 << pips) {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                game.turn.active_player = alice;
                game.turn.priority_player = Some(alice);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                let cost = ManaCost::from_pips(vec![vec![ManaSymbol::Green, other]; pips as usize]);
                let definition = CardDefinitionBuilder::new(
                    CardId::new(),
                    "Announced hybrid or Phyrexian spell",
                )
                .card_types(vec![CardType::Sorcery])
                .mana_cost(cost.clone())
                .build();
                let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
                game.player_mut(alice).unwrap().mana_pool.green = pips;
                if other == ManaSymbol::Red {
                    game.player_mut(alice).unwrap().mana_pool.red = pips;
                }
                if other == ManaSymbol::Generic(2) {
                    game.player_mut(alice).unwrap().mana_pool.colorless = 2 * pips;
                }
                let action = compute_legal_actions(&game, alice).unwrap().into_iter().find(|action|
                    matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
                let mut dm = Announce {
                    mask,
                    seen: 0,
                    pips,
                    pool_before: game.player(alice).unwrap().mana_pool.clone(),
                };
                let stack = audit_finish_cost_rule_cast_with(&mut game, action, &mut dm);
                assert_eq!(dm.seen, pips as usize);
                let second = mask.count_ones();
                assert_eq!(game.player(alice).unwrap().mana_pool.green, second);
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.red,
                    if other == ManaSymbol::Red {
                        pips - second
                    } else {
                        0
                    }
                );
                assert_eq!(
                    game.player(alice).unwrap().mana_pool.colorless,
                    if other == ManaSymbol::Generic(2) {
                        2 * (pips - second)
                    } else {
                        0
                    }
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if other == ManaSymbol::Life(2) {
                        20 - 2 * second as i32
                    } else {
                        20
                    }
                );
                assert_eq!(
                    game.object(stack)
                        .unwrap()
                        .mana_cost
                        .as_ref()
                        .unwrap()
                        .pips(),
                    cost.pips()
                );
            }
        }
    }
}

#[test]
fn cr_118_13a_mana_ability_phyrexian_choice_precedes_any_tap_cost_payment() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::{Ability, CardId, CardType, ManaCost, ManaSymbol, Zone};
    struct Announce {
        life: bool,
        tapped_at_choice: Vec<bool>,
    }
    impl DecisionMaker for Announce {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            assert_eq!(ctx.description, "Choose mana or life payment");
            self.tapped_at_choice
                .push(game.is_tapped(ctx.source.unwrap()));
            assert_eq!(game.player(ctx.player).unwrap().life, 20);
            assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, 1);
            let option = ctx
                .options
                .iter()
                .find(|option| {
                    option.description.to_ascii_lowercase().contains("life") == self.life
                })
                .unwrap();
            vec![option.index]
        }
    }
    let alice = PlayerId::from_index(0);
    let mut observed = Vec::new();
    for tap_first in [false, true] {
        for pay_life in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mana = Cost::mana(ManaCost::from_pips(vec![vec![
                ManaSymbol::Green,
                ManaSymbol::Life(2),
            ]]));
            let costs = if tap_first {
                vec![Cost::tap(), mana]
            } else {
                vec![mana, Cost::tap()]
            };
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Tap and Phyrexian mana activation")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::mana(
                        TotalCost::from_costs(costs),
                        vec![ManaSymbol::Red],
                    ))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.player_mut(alice).unwrap().mana_pool.green = 1;
            let mut dm = Announce {
                life: pay_life,
                tapped_at_choice: vec![],
            };
            ironsmith::special_actions::perform_activate_mana_ability(
                &mut game, alice, source, 0, &mut dm,
            )
            .unwrap();
            assert_eq!(dm.tapped_at_choice.len(), 1);
            observed.push(dm.tapped_at_choice[0]);
            assert!(game.is_tapped(source));
            assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
            assert_eq!(
                game.player(alice).unwrap().mana_pool.green,
                u32::from(pay_life)
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                if pay_life { 18 } else { 20 }
            );
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        observed,
        vec![false; 4],
        "CR 118.13a/602.2b/601.2b: mana-versus-life is announced while proposing the mana ability, before any cost is paid, regardless of component storage order"
    );
}

#[test]
fn cr_118_13c_turn_face_up_phyrexian_cost_is_chosen_before_payment_and_reveal() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cost::TotalCost;
    use ironsmith::game_state::Phase;
    use ironsmith::mana_payment::{ManaPaymentResponse, ManaPipId};
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
    use ironsmith::{
        Ability, CardId, CardType, ManaCost, ManaSymbol, PowerToughness, StaticAbility, Zone,
    };
    struct Choose {
        mask: u32,
        pips: u32,
        calls: usize,
    }
    impl DecisionMaker for Choose {
        fn decide_mana_payment(
            &mut self,
            game: &GameState,
            ctx: &ironsmith::decisions::context::ManaPaymentContext,
        ) -> ManaPaymentResponse {
            self.calls += 1;
            assert!(
                game.is_face_down(ctx.source),
                "Pay the special-action cost before revealing the permanent"
            );
            assert_eq!(game.player(ctx.player).unwrap().life, 20);
            assert_eq!(game.player(ctx.player).unwrap().mana_pool.green, self.pips);
            let desired = (0..self.pips)
                .filter(|index| self.mask & (1 << index) != 0)
                .map(ManaPipId)
                .collect::<Vec<_>>();
            if ctx.request.preferences.required_life_pips != desired {
                let mut preferences = ctx.request.preferences.clone();
                preferences.required_life_pips = desired;
                return ManaPaymentResponse::Replan { preferences };
            }
            assert!(ctx.plan.payable);
            assert_eq!(ctx.plan.life_to_pay, 2 * self.mask.count_ones());
            ManaPaymentResponse::Confirm {
                plan_id: ctx.plan.id,
                request_hash: ctx.plan.request_hash,
            }
        }
    }
    let alice = PlayerId::from_index(0);
    for pips in [1, 2] {
        for mask in 0..(1 << pips) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = alice;
            game.turn.priority_player = Some(alice);
            game.turn.phase = Phase::FirstMain;
            game.turn.step = None;
            let cost = ManaCost::from_pips(vec![
                vec![ManaSymbol::Green, ManaSymbol::Life(2)];
                pips as usize
            ]);
            let definition =
                CardDefinitionBuilder::new(CardId::new(), "Phyrexian turn-face-up cost")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(3, 4))
                    .with_ability(Ability::static_ability(StaticAbility::morph(
                        TotalCost::mana(cost),
                    )))
                    .build();
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            game.set_face_down(source);
            game.player_mut(alice).unwrap().mana_pool.green = pips;
            let mut dm = Choose {
                mask,
                pips,
                calls: 0,
            };
            ironsmith::special_actions::perform(
                SpecialAction::TurnFaceUp {
                    permanent_id: source,
                    method: TurnFaceUpMethod::TurnFaceUpAbility,
                },
                &mut game,
                alice,
                &mut dm,
            )
            .unwrap();
            assert_eq!(dm.calls, if mask == 0 { 1 } else { 2 });
            assert!(!game.is_face_down(source));
            assert_eq!(
                (game.current_power(source), game.current_toughness(source)),
                (Some(3), Some(4))
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                20 - 2 * mask.count_ones() as i32
            );
            assert_eq!(
                game.player(alice).unwrap().mana_pool.green,
                mask.count_ones()
            );
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_122_6_6a_entry_counters_count_as_placement_by_the_entering_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{
        MoveToZoneEffect, PutCountersEffect, RegisterEnterWithCountersReplacementEffect,
        ReplacementApplyMode,
    };
    use ironsmith::triggers::{CountMode, CounterPutOnTrigger, Trigger, TriggerQueue};
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, CounterType, Effect, ObjectFilter, PowerToughness,
        StackEntry, StaticAbility, Zone,
    };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for owner in [alice, bob] {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let mut observers = Vec::new();
            for player in [alice, bob] {
                let trigger = CounterPutOnTrigger::new(ObjectFilter::creature())
                    .counter_type(CounterType::Charge)
                    .source_controller(PlayerFilter::You)
                    .count(CountMode::OneOrMore);
                let definition =
                    CardDefinitionBuilder::new(CardId::new(), "Counter-placement actor observer")
                        .card_types(vec![CardType::Enchantment])
                        .with_ability(Ability::triggered(
                            Trigger::new(trigger),
                            vec![Effect::gain_life(3)],
                        ))
                        .build();
                observers.push(game.create_object_from_definition(
                    &definition,
                    player,
                    Zone::Battlefield,
                ));
            }
            let source = observers[0];
            let mut builder =
                CardDefinitionBuilder::new(CardId::new(), "Counter placement recipient")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(3, 4));
            if mode == 0 {
                builder = builder.with_ability(Ability::static_ability(
                    StaticAbility::enters_with_counters(CounterType::Charge, 2),
                ));
            }
            let card = game.create_object_from_definition(
                &builder.build(),
                owner,
                if mode == 2 {
                    Zone::Battlefield
                } else {
                    Zone::Hand
                },
            );
            let stable = game.object(card).unwrap().stable_id;
            if mode == 1 {
                RegisterEnterWithCountersReplacementEffect::new(
                    ObjectFilter::creature(),
                    CounterType::Charge,
                    ironsmith::effect::Value::Fixed(2),
                    ReplacementApplyMode::UntilEndOfTurn,
                )
                .execute(&mut game, &mut EffectContext::new_default(source, alice))
                .unwrap();
            }
            let effect = if mode == 2 {
                Effect::new(PutCountersEffect::new(
                    CounterType::Charge,
                    2,
                    ChooseSpec::SpecificObject(card),
                ))
            } else {
                Effect::new(
                    MoveToZoneEffect::new(
                        ChooseSpec::SpecificObject(card),
                        Zone::Battlefield,
                        false,
                    )
                    .under_owner_control(),
                )
            };
            game.stack
                .push(StackEntry::ability(source, alice, vec![effect]));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let recipient = game
                .battlefield
                .iter()
                .copied()
                .find(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.stable_id == stable)
                })
                .unwrap();
            assert_eq!(game.current_controller(recipient), Some(owner));
            assert_eq!(game.counter_count(recipient, CounterType::Charge), 2);
            let actor = if mode == 2 { alice } else { owner };
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
            assert_eq!(
                queue.entries.len(),
                1,
                "CR 122.6: one-or-more placement trigger includes entry counters; owner={owner:?}, mode={mode}"
            );
            assert_eq!(
                queue.entries[0].controller, actor,
                "CR 122.6a: no specified entry placer means entering controller; ordinary explicit effect uses its actor"
            );
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                game.player(alice).unwrap().life,
                if actor == alice { 23 } else { 20 }
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                if actor == bob { 23 } else { 20 }
            );
            assert!(game.stack.is_empty());
        }
    }
}

#[test]
fn cr_122_5_counter_move_doubling_changes_placement_without_extra_removal() {
    use ironsmith::effects::MoveCountersEffect;
    use ironsmith::events::counters::matchers::WouldPutCountersMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::{ChooseSpec, CounterType, ObjectFilter, Zone};
    let alice = PlayerId::from_index(0);
    for requested in [0, 1, 3, 5] {
        for double in [false, true] {
            for same_object in [false, true] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let from = counter_creature(&mut game, Zone::Battlefield);
                let to = if same_object {
                    from
                } else {
                    counter_creature(&mut game, Zone::Battlefield)
                };
                game.add_counters(from, CounterType::Charge, 3);
                game.add_counters(from, CounterType::Lore, 2);
                if !same_object {
                    game.add_counters(to, CounterType::Charge, 1);
                }
                if double {
                    game.effect_store.replacement_effects.add_effect(
                        ReplacementEffect::with_matcher(
                            from,
                            alice,
                            WouldPutCountersMatcher::new(
                                ObjectFilter::specific(to),
                                Some(CounterType::Charge),
                            ),
                            ReplacementAction::DoubleCounters {
                                counter_type: Some(CounterType::Charge),
                            },
                        ),
                    );
                }
                MoveCountersEffect::new(
                    CounterType::Charge,
                    requested,
                    ChooseSpec::SpecificObject(from),
                    ChooseSpec::SpecificObject(to),
                )
                .execute(&mut game, &mut EffectContext::new_default(from, alice))
                .unwrap();
                let moved = if same_object {
                    0
                } else {
                    requested.min(3) as u32
                };
                assert_eq!(
                    game.counter_count(from, CounterType::Charge),
                    3 - moved,
                    "CR 122.5: destination multiplier cannot remove extra donor counters; requested={requested}, doubled={double}, same={same_object}"
                );
                assert_eq!(
                    game.counter_count(to, CounterType::Charge),
                    if same_object {
                        3
                    } else {
                        1 + moved * if double { 2 } else { 1 }
                    },
                    "Moving to the same object is impossible; otherwise only the placement is multiplied"
                );
                assert_eq!(game.counter_count(from, CounterType::Lore), 2);
                if !same_object {
                    assert_eq!(game.counter_count(to, CounterType::Lore), 0);
                }
            }
        }
    }
}

#[test]
fn cr_122_1_counters_merge_by_kind_and_never_become_objects_or_tokens() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{
        CreateTokenEffect, PoisonCountersEffect, PutCountersEffect, RemoveCountersEffect,
    };
    use ironsmith::object::ObjectKind;
    use ironsmith::{CardId, CardType, ChooseSpec, CounterType, PowerToughness, Zone};
    let alice = PlayerId::from_index(0);
    for split in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = counter_creature(&mut game, Zone::Battlefield);
        let initial_ids = game
            .objects_in_deterministic_order()
            .iter()
            .map(|object| object.id)
            .collect::<Vec<_>>();
        for amount in if split { vec![2, 3] } else { vec![5] } {
            PutCountersEffect::new(
                CounterType::Charge,
                amount,
                ChooseSpec::SpecificObject(source),
            )
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        }
        PutCountersEffect::new(
            CounterType::PlusOnePlusOne,
            2,
            ChooseSpec::SpecificObject(source),
        )
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
        PoisonCountersEffect::new(1, PlayerFilter::Specific(alice))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        PoisonCountersEffect::new(2, PlayerFilter::Specific(alice))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        assert_eq!(game.player(alice).unwrap().poison_counters, 3);
        assert_eq!(
            (game.current_power(source), game.current_toughness(source)),
            (Some(5), Some(6))
        );
        assert_eq!(game.counter_count(source, CounterType::Charge), 5);
        assert_eq!(game.object(source).unwrap().counters.counts().len(), 2);
        RemoveCountersEffect::new(CounterType::Charge, 2, ChooseSpec::SpecificObject(source))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(
            game.objects_in_deterministic_order()
                .iter()
                .map(|object| object.id)
                .collect::<Vec<_>>(),
            initial_ids,
            "CR 122.1: neither object counters nor player counters create game objects"
        );
        assert!(
            game.objects_in_deterministic_order()
                .iter()
                .all(|object| object.kind != ObjectKind::Token)
        );
        let blueprint =
            CardDefinitionBuilder::new(CardId::new(), "Token distinct from all counters")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
        CreateTokenEffect::one(blueprint)
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        let tokens = game
            .objects_in_deterministic_order()
            .into_iter()
            .filter(|object| object.kind == ObjectKind::Token)
            .map(|object| object.id)
            .collect::<Vec<_>>();
        assert_eq!(tokens.len(), 1);
        let token = tokens[0];
        assert!(game.object(token).unwrap().counters.counts().is_empty());
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.player(alice).unwrap().poison_counters, 3);
        PutCountersEffect::new(CounterType::Charge, 1, ChooseSpec::SpecificObject(token))
            .execute(&mut game, &mut EffectContext::new_default(source, alice))
            .unwrap();
        assert_eq!(game.counter_count(token, CounterType::Charge), 1);
        assert_eq!(game.object(token).unwrap().kind, ObjectKind::Token);
        assert_eq!(
            game.objects_in_deterministic_order().len(),
            initial_ids.len() + 1,
            "A token can carry a counter while remaining a distinct object; the counter adds no object"
        );
    }
}

#[test]
fn cr_123_5a_meld_retains_stickers_from_both_components_in_timestamp_order() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::{MeldEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, PowerToughness, StackEntry, StaticAbility,
        Zone,
    };
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for component in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let definitions = [
            ("Graf Rats", 2, 1),
            ("Midnight Scavengers", 3, 3),
            ("Chittering Host", 5, 6),
        ]
        .into_iter()
        .map(|(name, power, toughness)| {
            CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(power, toughness))
                .build()
        })
        .collect::<Vec<_>>();
        for definition in &definitions {
            game.register_linked_face_definition(definition);
        }
        let source = game.create_object_from_definition(&definitions[0], alice, Zone::Battlefield);
        let counterpart =
            game.create_object_from_definition(&definitions[1], alice, Zone::Battlefield);
        for (index, host) in [source, counterpart].into_iter().enumerate() {
            if component != 2 && component != index {
                continue;
            }
            game.add_accessible_name_sticker(alice, if index == 0 { "Old" } else { "New" });
            game.add_accessible_pt_sticker(
                alice,
                0,
                if index == 0 { 6 } else { 8 },
                if index == 0 { 7 } else { 9 },
            );
            game.add_accessible_ability_sticker(
                alice,
                0,
                vec![Ability::static_ability(StaticAbility::flying())],
                "Flying",
            );
            game.add_accessible_art_sticker(alice);
            for action in [
                KeywordActionKind::NameSticker,
                KeywordActionKind::PowerToughnessSticker,
                KeywordActionKind::AbilitySticker,
                KeywordActionKind::ArtSticker,
            ] {
                PutStickerEffect::new(ChooseSpec::SpecificObject(host), action)
                    .execute(&mut game, &mut EffectContext::new_default(source, alice))
                    .unwrap();
            }
            assert_eq!(
                game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
                4
            );
        }
        game.stack.push(StackEntry::ability(
            source,
            alice,
            vec![Effect::new(MeldEffect::new("Chittering Host"))],
        ));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.battlefield.len(),
            1,
            "Native meld must form a single permanent before testing stickers"
        );
        let melded = game.battlefield[0];
        assert_eq!(
            game.object(melded).unwrap().name.as_str(),
            "Chittering Host"
        );
        assert_eq!(
            game.melded_permanent(game.object(melded).unwrap().stable_id)
                .unwrap()
                .components
                .len(),
            2
        );
        actual.push((
            game.sticker_count_on_object(melded, KeywordActionKind::Sticker, None),
            game.current_name(melded).unwrap(),
            game.current_power(melded),
            game.current_toughness(melded),
            game.object_has_ability(melded, &StaticAbility::flying()),
        ));
        expected.push((
            if component == 2 { 8 } else { 4 },
            match component {
                0 => "Old Chittering Host",
                1 => "New Chittering Host",
                _ => "New Old Chittering Host",
            }
            .to_string(),
            Some(if component == 0 { 6 } else { 8 }),
            Some(if component == 0 { 7 } else { 9 }),
            true,
        ));
        assert!(game.stack.is_empty());
    }
    assert_eq!(
        actual, expected,
        "CR 123.5a: all stickers on either meld component apply to the resulting permanent, preserving relative timestamps"
    );
}

#[test]
fn cr_123_5b_mutation_keeps_stickers_on_every_component() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::PutStickerEffect;
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, CardId, CardType, ChooseSpec, PowerToughness, StaticAbility, Zone};
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for spell_on_top in [false, true] {
        for component in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host_definition = CardDefinitionBuilder::new(CardId::new(), "Recipient Host")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(3, 4))
                .build();
            let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Mutating Visitor")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(5, 6))
                .build();
            let host =
                game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);
            let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
            for (index, object) in [host, spell].into_iter().enumerate() {
                if component != 2 && component != index {
                    continue;
                }
                game.add_accessible_name_sticker(alice, if index == 0 { "Old" } else { "New" });
                game.add_accessible_pt_sticker(
                    alice,
                    0,
                    if index == 0 { 6 } else { 8 },
                    if index == 0 { 7 } else { 9 },
                );
                game.add_accessible_ability_sticker(
                    alice,
                    0,
                    vec![Ability::static_ability(StaticAbility::flying())],
                    "Flying",
                );
                game.add_accessible_art_sticker(alice);
                for action in [
                    KeywordActionKind::NameSticker,
                    KeywordActionKind::PowerToughnessSticker,
                    KeywordActionKind::AbilitySticker,
                    KeywordActionKind::ArtSticker,
                ] {
                    PutStickerEffect::new(ChooseSpec::SpecificObject(object), action)
                        .execute(&mut game, &mut EffectContext::new_default(host, alice))
                        .unwrap();
                }
                assert_eq!(
                    game.sticker_count_on_object(object, KeywordActionKind::Sticker, None),
                    4
                );
            }
            assert_eq!(
                game.merge_mutating_creature_spell(spell, host, spell_on_top),
                Some(host)
            );
            assert_eq!(game.battlefield.len(), 1);
            assert_eq!(
                game.merged_permanent(game.object(host).unwrap().stable_id)
                    .unwrap()
                    .components
                    .len(),
                2
            );
            actual.push((
                game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
                game.current_name(host).unwrap(),
                game.current_power(host),
                game.current_toughness(host),
                game.object_has_ability(host, &StaticAbility::flying()),
            ));
            let base = if spell_on_top {
                "Mutating Visitor"
            } else {
                "Recipient Host"
            };
            let prefix = match component {
                0 => "Old",
                1 => "New",
                _ => "New Old",
            };
            expected.push((
                if component == 2 { 8 } else { 4 },
                format!("{prefix} {base}"),
                Some(if component == 0 { 6 } else { 8 }),
                Some(if component == 0 { 7 } else { 9 }),
                true,
            ));
        }
    }
    assert_eq!(
        actual, expected,
        "CR 123.5b: incoming and existing components' stickers belong to the merged permanent, regardless of which card is on top"
    );
}

fn audit_create_unstickered_meld(game: &mut GameState, owner: PlayerId) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::MeldEffect;
    use ironsmith::{CardId, CardType, PowerToughness, Zone};
    let definitions = [
        ("Graf Rats", 2, 1),
        ("Midnight Scavengers", 3, 3),
        ("Chittering Host", 5, 6),
    ]
    .into_iter()
    .map(|(name, power, toughness)| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    })
    .collect::<Vec<_>>();
    for definition in &definitions {
        game.register_linked_face_definition(definition);
    }
    let source = game.create_object_from_definition(&definitions[0], owner, Zone::Battlefield);
    game.create_object_from_definition(&definitions[1], owner, Zone::Battlefield);
    MeldEffect::new("Chittering Host")
        .execute(game, &mut EffectContext::new_default(source, owner))
        .unwrap();
    assert_eq!(game.battlefield.len(), 1);
    let melded = game.battlefield[0];
    assert_eq!(
        game.melded_permanent(game.object(melded).unwrap().stable_id)
            .unwrap()
            .components
            .len(),
        2
    );
    melded
}

#[test]
fn cr_123_5c_meld_public_departure_retains_all_stickers_on_the_owners_chosen_card() {
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effects::{MoveToZoneEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{Ability, ChooseSpec, Effect, ObjectId, StackEntry, StaticAbility, Zone};
    struct ChooseCard {
        name: &'static str,
    }
    impl DecisionMaker for ChooseCard {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.description.contains(self.name))
                    .unwrap_or_else(|| ctx.options.iter().find(|option| option.legal).unwrap())
                    .index,
            ]
        }
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            vec![
                ctx.candidates
                    .iter()
                    .find(|candidate| {
                        candidate.legal
                            && game
                                .object(candidate.id)
                                .is_some_and(|object| object.name.as_str() == self.name)
                    })
                    .unwrap_or_else(|| {
                        ctx.candidates
                            .iter()
                            .find(|candidate| candidate.legal)
                            .unwrap()
                    })
                    .id,
            ]
        }
    }
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for destination in [Zone::Graveyard, Zone::Exile, Zone::Command] {
        for chosen in ["Graf Rats", "Midnight Scavengers"] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let melded = audit_create_unstickered_meld(&mut game, alice);
            game.add_accessible_name_sticker(alice, "Bright");
            game.add_accessible_pt_sticker(alice, 0, 8, 9);
            game.add_accessible_ability_sticker(
                alice,
                0,
                vec![Ability::static_ability(StaticAbility::flying())],
                "Flying",
            );
            game.add_accessible_art_sticker(alice);
            for action in [
                KeywordActionKind::NameSticker,
                KeywordActionKind::PowerToughnessSticker,
                KeywordActionKind::AbilitySticker,
                KeywordActionKind::ArtSticker,
            ] {
                PutStickerEffect::new(ChooseSpec::SpecificObject(melded), action)
                    .execute(&mut game, &mut EffectContext::new_default(melded, alice))
                    .unwrap();
            }
            assert_eq!(
                game.sticker_count_on_object(melded, KeywordActionKind::Sticker, None),
                4
            );
            let mut dm = ChooseCard { name: chosen };
            game.stack.push(StackEntry::ability(
                melded,
                alice,
                vec![Effect::new(MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(melded),
                    destination,
                    false,
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let cards = game.objects_in_zone(destination);
            assert_eq!(
                cards.len(),
                2,
                "Native meld departure must return both component cards"
            );
            for name in ["Graf Rats", "Midnight Scavengers"] {
                let id = *cards
                    .iter()
                    .find(|id| game.object(**id).unwrap().name.as_str() == name)
                    .unwrap();
                actual.push((
                    game.sticker_count_on_object(id, KeywordActionKind::Sticker, None),
                    game.current_name(id).unwrap(),
                    game.current_power(id),
                    game.current_toughness(id),
                    game.object_has_ability(id, &StaticAbility::flying()),
                ));
                let selected = name == chosen;
                expected.push((
                    if selected { 4 } else { 0 },
                    if selected {
                        format!("Bright {name}")
                    } else {
                        name.to_string()
                    },
                    Some(if selected {
                        8
                    } else if name == "Graf Rats" {
                        2
                    } else {
                        3
                    }),
                    Some(if selected {
                        9
                    } else if name == "Graf Rats" {
                        1
                    } else {
                        3
                    }),
                    selected,
                ));
            }
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual, expected,
        "CR 123.5c: owner chooses exactly one resulting card to retain every sticker and its effects on a public-zone departure"
    );
}

#[test]
fn cr_123_5c_merged_public_departure_puts_stickers_on_one_chosen_component() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::effects::{MoveToZoneEffect, PutStickerEffect};
    use ironsmith::events::KeywordActionKind;
    use ironsmith::{
        Ability, CardId, CardType, ChooseSpec, Effect, ObjectId, PowerToughness, StackEntry,
        StaticAbility, Zone,
    };
    struct ChooseCard {
        name: &'static str,
    }
    impl DecisionMaker for ChooseCard {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            vec![
                ctx.options
                    .iter()
                    .find(|option| option.description.contains(self.name))
                    .unwrap_or_else(|| ctx.options.iter().find(|option| option.legal).unwrap())
                    .index,
            ]
        }
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            vec![
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal && candidate.name.contains(self.name))
                    .unwrap_or_else(|| {
                        ctx.candidates
                            .iter()
                            .find(|candidate| candidate.legal)
                            .unwrap()
                    })
                    .id,
            ]
        }
    }
    let alice = PlayerId::from_index(0);
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for destination in [Zone::Graveyard, Zone::Exile, Zone::Command] {
        for chosen in ["Recipient Host", "Mutating Visitor"] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let host_definition = CardDefinitionBuilder::new(CardId::new(), "Recipient Host")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(3, 4))
                .build();
            let spell_definition = CardDefinitionBuilder::new(CardId::new(), "Mutating Visitor")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(5, 6))
                .build();
            let host =
                game.create_object_from_definition(&host_definition, alice, Zone::Battlefield);
            let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
            assert_eq!(
                game.merge_mutating_creature_spell(spell, host, true),
                Some(host)
            );
            assert_eq!(
                game.merged_permanent(game.object(host).unwrap().stable_id)
                    .unwrap()
                    .components
                    .len(),
                2
            );
            game.add_accessible_name_sticker(alice, "Bright");
            game.add_accessible_pt_sticker(alice, 0, 8, 9);
            game.add_accessible_ability_sticker(
                alice,
                0,
                vec![Ability::static_ability(StaticAbility::flying())],
                "Flying",
            );
            game.add_accessible_art_sticker(alice);
            for action in [
                KeywordActionKind::NameSticker,
                KeywordActionKind::PowerToughnessSticker,
                KeywordActionKind::AbilitySticker,
                KeywordActionKind::ArtSticker,
            ] {
                PutStickerEffect::new(ChooseSpec::SpecificObject(host), action)
                    .execute(&mut game, &mut EffectContext::new_default(host, alice))
                    .unwrap();
            }
            assert_eq!(
                game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
                4
            );
            game.stack.push(StackEntry::ability(
                host,
                alice,
                vec![Effect::new(MoveToZoneEffect::new(
                    ChooseSpec::SpecificObject(host),
                    destination,
                    false,
                ))],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut ChooseCard { name: chosen },
            )
            .unwrap();
            let cards = game.objects_in_zone(destination);
            assert_eq!(cards.len(), 2);
            for name in ["Recipient Host", "Mutating Visitor"] {
                let id = *cards
                    .iter()
                    .find(|id| game.object(**id).unwrap().name.as_str() == name)
                    .unwrap();
                actual.push((
                    game.sticker_count_on_object(id, KeywordActionKind::Sticker, None),
                    game.current_name(id).unwrap(),
                    game.current_power(id),
                    game.current_toughness(id),
                    game.object_has_ability(id, &StaticAbility::flying()),
                ));
                let selected = name == chosen;
                expected.push((
                    if selected { 4 } else { 0 },
                    if selected {
                        format!("Bright {name}")
                    } else {
                        name.to_string()
                    },
                    Some(if selected {
                        8
                    } else if name == "Recipient Host" {
                        3
                    } else {
                        5
                    }),
                    Some(if selected {
                        9
                    } else if name == "Recipient Host" {
                        4
                    } else {
                        6
                    }),
                    selected,
                ));
            }
            assert!(game.stack.is_empty());
        }
    }
    assert_eq!(
        actual, expected,
        "CR 123.5c: a merged permanent's owner chooses one component for all stickers, and each sticker's effects continue on that card"
    );
}

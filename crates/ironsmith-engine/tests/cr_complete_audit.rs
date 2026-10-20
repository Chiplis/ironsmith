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

use super::*;
use crate::StaticAbility;
use crate::card::{CardBuilder, PowerToughness, PtValue};
use crate::cards::CardDefinitionBuilder;
use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::EventContext;
use crate::events::cause::EventCause;
use crate::events::processing::{EventOutcome, process_damage_assignments_with_event};
use crate::events::zones::ZoneChangeEvent;
use crate::ids::{CardId, PlayerId};
use crate::mana::ManaCost;
use crate::rules::state_based::apply_state_based_actions_with;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[test]
fn test_doesnt_untap() {
    let ability = DoesntUntap;
    assert_eq!(ability.id(), StaticAbilityId::DoesntUntap);
    assert!(ability.affects_untap());
}

#[test]
fn test_may_choose_not_to_untap_during_untap_step() {
    let ability = MayChooseNotToUntapDuringUntapStep::new("this artifact");
    assert_eq!(
        ability.id(),
        StaticAbilityId::MayChooseNotToUntapDuringUntapStep
    );
    assert_eq!(
        ability.display(),
        "You may choose not to untap this artifact during your untap step"
    );
}

#[test]
fn test_enters_tapped() {
    let ability = EntersTapped;
    assert_eq!(ability.id(), StaticAbilityId::EntersTapped);
    assert!(ability.enters_tapped());
}

#[test]
fn played_by_opponents_entry_filter_preserves_authored_surface() {
    let mut filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::Opponent)
        .in_zone(Zone::Battlefield);
    filter.set_played_by_opponent_surface(ironsmith_core::PlayedByOpponentSurface::YourOpponents);
    let ability = EnterTappedForFilter::new(filter);

    assert_eq!(
        ability.display(),
        "Creatures played by your opponents enter tapped"
    );
}

#[test]
fn conditional_other_permanent_entry_rules_follow_source_status_and_exclude_source() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_card = CardBuilder::new(CardId::from_raw(6001), "Entry Rule Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build();
    let incoming_card = CardBuilder::new(CardId::from_raw(6002), "Incoming Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let incoming = game.create_object_from_card(&incoming_card, alice, Zone::Hand);

    let mut other_permanents = ObjectFilter::default().in_zone(Zone::Battlefield);
    other_permanents.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    other_permanents.other = true;

    let tapped_rule = EnterTappedForFilter::new(other_permanents.clone())
        .with_condition(Condition::SourceIsTapped);
    let untapped_rule =
        EnterUntappedForFilter::new(other_permanents).with_condition(Condition::SourceIsUntapped);
    assert_eq!(
        tapped_rule.display(),
        "As long as this creature is tapped, other permanents enter tapped"
    );
    assert_eq!(
        untapped_rule.display(),
        "As long as this creature is untapped, other permanents enter untapped"
    );

    let tapped_replacement = tapped_rule
        .generate_replacement_effect(source, alice)
        .expect("tapped entry rule should generate a replacement");
    let untapped_replacement = untapped_rule
        .generate_replacement_effect(source, alice)
        .expect("untapped entry rule should generate a replacement");
    let incoming_event = ZoneChangeEvent::with_cause(
        incoming,
        Zone::Hand,
        Zone::Battlefield,
        EventCause::effect(),
        None,
    );

    {
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(
            !tapped_replacement
                .matcher
                .as_ref()
                .expect("tapped rule matcher")
                .matches_event(&incoming_event, &ctx),
            "the tapped rule must be inactive while its source is untapped"
        );
        assert!(
            untapped_replacement
                .matcher
                .as_ref()
                .expect("untapped rule matcher")
                .matches_event(&incoming_event, &ctx),
            "the untapped rule must be active while its source is untapped"
        );
    }

    game.tap(source);
    let ctx = EventContext::for_replacement_effect(alice, source, &game);
    assert!(
        tapped_replacement
            .matcher
            .as_ref()
            .expect("tapped rule matcher")
            .matches_event(&incoming_event, &ctx),
        "the tapped rule must become active when its source is tapped"
    );
    assert!(
        !untapped_replacement
            .matcher
            .as_ref()
            .expect("untapped rule matcher")
            .matches_event(&incoming_event, &ctx),
        "the untapped rule must become inactive when its source is tapped"
    );

    let source_event = ZoneChangeEvent::with_cause(
        source,
        Zone::Hand,
        Zone::Battlefield,
        EventCause::effect(),
        None,
    );
    assert!(
        !tapped_replacement
            .matcher
            .as_ref()
            .expect("tapped rule matcher")
            .matches_event(&source_event, &ctx),
        "other permanents must exclude the entry-rule source itself"
    );
}

#[test]
fn test_no_maximum_hand_size() {
    let ability = NoMaximumHandSize;
    assert_eq!(ability.id(), StaticAbilityId::NoMaximumHandSize);

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = ObjectId::from_raw(42);
    ability.apply_restrictions(&mut game, source, alice);
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .max_hand_size,
        i32::MAX
    );
}

#[test]
fn test_set_maximum_hand_size_for_you() {
    let ability = SetMaximumHandSize::new(PlayerFilter::You, 20);
    assert_eq!(ability.id(), StaticAbilityId::SetMaximumHandSize);
    assert_eq!(ability.display(), "Your maximum hand size is twenty.");

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = ObjectId::from_raw(43);
    ability.apply_restrictions(&mut game, source, alice);

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .max_hand_size,
        20
    );
    assert_eq!(game.player(bob).expect("bob should exist").max_hand_size, 7);
}

#[test]
fn test_reduce_maximum_hand_size_for_opponents() {
    let ability = ReduceMaximumHandSize::new(PlayerFilter::Opponent, 4);
    assert_eq!(ability.id(), StaticAbilityId::ReduceMaximumHandSize);

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = ObjectId::from_raw(43);
    ability.apply_restrictions(&mut game, source, alice);

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .max_hand_size,
        7
    );
    assert_eq!(game.player(bob).expect("bob should exist").max_hand_size, 3);
}

#[test]
fn test_increase_maximum_hand_size_for_you() {
    let ability = IncreaseMaximumHandSize::new(PlayerFilter::You, 2);
    assert_eq!(ability.id(), StaticAbilityId::IncreaseMaximumHandSize);
    assert_eq!(
        ability.display(),
        "Your maximum hand size is increased by two."
    );

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = ObjectId::from_raw(44);
    ability.apply_restrictions(&mut game, source, alice);

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .max_hand_size,
        9
    );
    assert_eq!(game.player(bob).expect("bob should exist").max_hand_size, 7);
}

#[test]
fn test_conditional_spell_keyword_active_by_mana_values() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    for (idx, mv) in [1u8, 2, 3, 4, 5].into_iter().enumerate() {
        let card = crate::card::CardBuilder::new(
            crate::ids::CardId::from_raw(800 + idx as u32),
            &format!("MV{mv}"),
        )
        .card_types(vec![crate::types::CardType::Instant])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mv),
        ]]))
        .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    let spec = ConditionalSpellKeywordSpec {
        keyword: ConditionalSpellKeywordKind::Flash,
        metric: GraveyardCountMetric::ManaValues,
        threshold: 5,
    };
    assert!(
        conditional_spell_keyword_active(spec, &game, alice),
        "expected mana-value threshold to be active"
    );
}

#[test]
fn test_this_spell_cast_restriction_kind_roundtrip() {
    let ability = ThisSpellCastRestriction::new(
        ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step(
        ),
        "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
    );
    assert_eq!(ability.id(), StaticAbilityId::ThisSpellCastRestriction);
    assert_eq!(
            ability.this_spell_cast_restriction_kind(),
            Some(ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step())
        );
}

#[test]
fn test_maximum_hand_size_seven_minus_card_types_applies_only_at_threshold() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ability = MaximumHandSizeSevenMinusYourGraveyardCardTypes::new(PlayerFilter::Opponent, 4);
    let source = ObjectId::from_raw(900);

    for (idx, card_type) in [
        crate::types::CardType::Artifact,
        crate::types::CardType::Creature,
        crate::types::CardType::Enchantment,
    ]
    .into_iter()
    .enumerate()
    {
        let card = crate::card::CardBuilder::new(
            crate::ids::CardId::from_raw(900 + idx as u32),
            &format!("Type{idx}"),
        )
        .card_types(vec![card_type])
        .build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }

    ability.apply_restrictions(&mut game, source, alice);
    assert_eq!(
        game.player(bob).expect("bob should exist").max_hand_size,
        7,
        "threshold not met: max hand size should remain default"
    );

    let fourth = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(999), "Type4")
        .card_types(vec![crate::types::CardType::Land])
        .build();
    game.create_object_from_card(&fourth, alice, Zone::Graveyard);

    ability.apply_restrictions(&mut game, source, alice);
    assert_eq!(
        game.player(bob).expect("bob should exist").max_hand_size,
        3,
        "with four card types, max hand size should be seven minus four"
    );
}

#[test]
fn test_draw_replacement_exile_top_face_down() {
    let ability = DrawReplacementExileTopFaceDown;
    assert_eq!(
        ability.id(),
        StaticAbilityId::DrawReplacementExileTopFaceDown
    );

    let replacement = ability
        .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
        .expect("draw replacement should create replacement effect");
    let ReplacementAction::Instead(effects) = &replacement.replacement else {
        panic!("expected draw replacement to use an Instead action");
    };
    assert_eq!(effects.len(), 2, "expected choose+exile effect sequence");
    let choose_debug = format!("{:?}", effects[0]);
    assert!(
        choose_debug.contains("ChooseObjectsEffect"),
        "expected first effect to choose top library card, got {choose_debug}"
    );
    assert!(
        !choose_debug.contains("RevealTopEffect"),
        "draw replacement should not reveal the card, got {choose_debug}"
    );
}

#[test]
fn test_draw_replacement_double() {
    let ability = DrawReplacementDouble;
    assert_eq!(ability.id(), StaticAbilityId::DrawReplacementDouble);

    let replacement = ability
        .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
        .expect("draw replacement should create replacement effect");
    let ReplacementAction::Instead(effects) = replacement.replacement else {
        panic!("expected draw replacement to execute nested draw effects");
    };
    assert_eq!(effects.len(), 1);
    assert!(
        format!("{:?}", effects[0]).contains("DrawCardsEffect"),
        "expected nested draw effect, got {:?}",
        effects[0]
    );
}

#[test]
fn test_draw_replacement_exile_top_and_play_sequence() {
    let ability = DrawReplacementExileTopAndPlay::new(2);
    assert_eq!(
        ability.id(),
        StaticAbilityId::DrawReplacementExileTopAndPlay
    );

    let replacement = ability
        .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
        .expect("draw replacement should create replacement effect");
    let ReplacementAction::Instead(effects) = replacement.replacement else {
        panic!("expected draw replacement to use an Instead action");
    };

    assert_eq!(effects.len(), 3, "expected choose+exile+grant sequence");
    assert!(
        ability.display().contains("top 2 cards"),
        "expected display text to keep top-two count"
    );
    assert!(
        format!("{:?}", effects[0]).contains("draw_replacement_top_cards"),
        "expected choose effect to tag exiled cards for follow-up play permission"
    );
    assert!(
        format!("{:?}", effects[2]).contains("GrantPlayTaggedEffect"),
        "expected final effect to grant play permission"
    );
}

#[test]
fn test_modify_damage_amount_replacement_respects_max_speed_condition() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let source_card = CardBuilder::new(CardId::from_raw(4300), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(2)))
        .build();
    let target_card = CardBuilder::new(CardId::from_raw(4301), "Bob Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let target = game.create_object_from_card(&target_card, bob, Zone::Battlefield);

    let condition = crate::ConditionExpr::ValueComparison {
        left: Value::Speed(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(4),
    };
    let ability = ModifyDamageAmountReplacement::new(
            ObjectFilter::default().you_control(),
            Some(PlayerFilter::Opponent),
            Some(ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent)),
            1,
            "If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals that much damage plus 1 instead.",
        )
        .with_condition(condition);
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("damage amount replacement should generate a replacement effect");
    game.effect_store
        .replacement_effects
        .add_resolution_effect(replacement);

    let before_max_speed = process_damage_assignments_with_event(
        &mut game,
        source,
        DamageTarget::Player(bob),
        2,
        false,
        EventCause::from_effect(source, alice),
    );
    assert_eq!(before_max_speed.assignments[0].amount, 2);

    game.start_engines(alice);
    for _ in 0..3 {
        game.increase_speed(alice, 1);
    }

    let player_damage = process_damage_assignments_with_event(
        &mut game,
        source,
        DamageTarget::Player(bob),
        2,
        false,
        EventCause::from_effect(source, alice),
    );
    assert_eq!(player_damage.assignments[0].amount, 3);

    let permanent_damage = process_damage_assignments_with_event(
        &mut game,
        source,
        DamageTarget::Object(target),
        4,
        false,
        EventCause::from_effect(source, alice),
    );
    assert_eq!(permanent_damage.assignments[0].amount, 5);
}

#[test]
fn harsh_judgment_redirects_chosen_color_spell_damage_to_source_controller() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let harsh_judgment = CardBuilder::new(CardId::from_raw(19001), "Harsh Judgment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let harsh_judgment_id = game.create_object_from_card(&harsh_judgment, alice, Zone::Battlefield);
    game.set_chosen_color(harsh_judgment_id, Color::White);

    let white_spell = CardBuilder::new(CardId::from_raw(19002), "White Instant")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::White,
        ]]))
        .card_types(vec![CardType::Instant])
        .build();
    let white_spell_id = game.create_object_from_card(&white_spell, bob, Zone::Stack);
    assert_eq!(game.current_controller(white_spell_id), Some(bob));

    let ability = RedirectDamageToSourceController::new(
        ObjectFilter::instant_or_sorcery().of_chosen_color(),
        PlayerFilter::You,
        "If instant or sorcery spell of the chosen color would deal damage to you, it deals that damage to its controller instead.",
    );
    let replacement = ability
        .generate_replacement_effect(harsh_judgment_id, alice)
        .expect("Harsh Judgment should generate a replacement effect");
    let matching_event = DamageEvent::with_cause(
        white_spell_id,
        DamageTarget::Player(alice),
        4,
        false,
        EventCause::from_effect(white_spell_id, bob),
    );
    let matching_ctx = EventContext::for_replacement_effect(alice, harsh_judgment_id, &game);
    assert!(
        replacement
            .matcher
            .as_ref()
            .expect("replacement should have a matcher")
            .matches_event(&matching_event, &matching_ctx),
        "Harsh Judgment replacement should match chosen-color spell damage to you"
    );
    game.effect_store
        .replacement_effects
        .add_resolution_effect(replacement);

    let damage = process_damage_assignments_with_event(
        &mut game,
        white_spell_id,
        DamageTarget::Player(alice),
        4,
        false,
        EventCause::from_effect(white_spell_id, bob),
    );

    assert_eq!(damage.assignments.len(), 1);
    assert_eq!(damage.assignments[0].target, DamageTarget::Player(bob));
    assert_eq!(damage.assignments[0].amount, 4);
}

#[test]
fn harsh_judgment_handles_sorcery_nonchosen_color_and_other_target_branches() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let harsh_judgment = CardBuilder::new(CardId::from_raw(19003), "Harsh Judgment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let harsh_judgment_id = game.create_object_from_card(&harsh_judgment, alice, Zone::Battlefield);
    game.set_chosen_color(harsh_judgment_id, Color::White);

    let red_spell = CardBuilder::new(CardId::from_raw(19004), "Red Instant")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Red,
        ]]))
        .card_types(vec![CardType::Instant])
        .build();
    let red_spell_id = game.create_object_from_card(&red_spell, bob, Zone::Stack);

    let white_spell = CardBuilder::new(CardId::from_raw(19005), "White Sorcery")
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::White,
        ]]))
        .card_types(vec![CardType::Sorcery])
        .build();
    let white_spell_id = game.create_object_from_card(&white_spell, bob, Zone::Stack);

    let target_creature = CardBuilder::new(CardId::from_raw(19006), "Target Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_creature_id =
        game.create_object_from_card(&target_creature, alice, Zone::Battlefield);

    let ability = RedirectDamageToSourceController::new(
        ObjectFilter::instant_or_sorcery().of_chosen_color(),
        PlayerFilter::You,
        "If instant or sorcery spell of the chosen color would deal damage to you, it deals that damage to its controller instead.",
    );
    let replacement = ability
        .generate_replacement_effect(harsh_judgment_id, alice)
        .expect("Harsh Judgment should generate a replacement effect");
    game.effect_store
        .replacement_effects
        .add_resolution_effect(replacement);

    let nonchosen_damage = process_damage_assignments_with_event(
        &mut game,
        red_spell_id,
        DamageTarget::Player(alice),
        3,
        false,
        EventCause::from_effect(red_spell_id, bob),
    );
    assert_eq!(nonchosen_damage.assignments.len(), 1);
    assert_eq!(
        nonchosen_damage.assignments[0].target,
        DamageTarget::Player(alice)
    );
    assert_eq!(nonchosen_damage.assignments[0].amount, 3);

    let other_target_damage = process_damage_assignments_with_event(
        &mut game,
        white_spell_id,
        DamageTarget::Object(target_creature_id),
        2,
        false,
        EventCause::from_effect(white_spell_id, bob),
    );
    assert_eq!(other_target_damage.assignments.len(), 1);
    assert_eq!(
        other_target_damage.assignments[0].target,
        DamageTarget::Object(target_creature_id)
    );
    assert_eq!(other_target_damage.assignments[0].amount, 2);

    let chosen_sorcery_damage = process_damage_assignments_with_event(
        &mut game,
        white_spell_id,
        DamageTarget::Player(alice),
        5,
        false,
        EventCause::from_effect(white_spell_id, bob),
    );
    assert_eq!(chosen_sorcery_damage.assignments.len(), 1);
    assert_eq!(
        chosen_sorcery_damage.assignments[0].target,
        DamageTarget::Player(bob)
    );
    assert_eq!(chosen_sorcery_damage.assignments[0].amount, 5);
}

#[test]
fn test_exile_to_countered_exile_instead_of_graveyard_generates_replacement() {
    let ability =
        ExileToCounteredExileInsteadOfGraveyard::new(PlayerFilter::Opponent, CounterType::Void);
    assert_eq!(
        ability.id(),
        StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
    );

    let replacement = ability
        .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
        .expect("replacement should be generated");
    let ReplacementAction::ExileWithSourceLinkCountersThen { counters, effects } =
        &replacement.replacement
    else {
        panic!("expected opponent-graveyard replacement to exile with source-linked counters");
    };
    assert_eq!(counters.as_slice(), &[(CounterType::Void, 1)]);
    assert!(effects.is_empty());
}

#[test]
fn exile_would_die_follow_up_exiles_matching_creature_and_creates_token_only_then() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let zombie = CardDefinitionBuilder::new(CardId::new(), "Zombie")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source_card = CardDefinitionBuilder::new(CardId::new(), "Exile Replacement Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::exile_would_die_instead_with_damage_source_and_follow_up(
                ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent)
                    .in_zone(Zone::Battlefield),
                None,
                vec![crate::effect::Effect::create_tokens(zombie, 1)],
            ),
        ))
        .build();
    let source = game.create_object_from_definition(&source_card, alice, Zone::Battlefield);
    let creature_card = CardDefinitionBuilder::new(CardId::new(), "Doomed Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let opposing_creature =
        game.create_object_from_definition(&creature_card, bob, Zone::Battlefield);
    let own_creature = game.create_object_from_definition(&creature_card, alice, Zone::Battlefield);
    let zombie_count = |game: &GameState| {
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| object.name == "Zombie")
            .count()
    };

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let matching = crate::events::processing::process_zone_change(
        &mut game,
        opposing_creature,
        Zone::Battlefield,
        Zone::Graveyard,
        EventCause::from_effect(source, alice),
        &mut dm,
    );
    assert!(matching.is_replaced(), "matching death={matching:?}");
    assert_eq!(game.objects_in_zone(Zone::Exile).len(), 1);
    assert_eq!(zombie_count(&game), 1);

    let nonmatching = crate::events::processing::process_zone_change(
        &mut game,
        own_creature,
        Zone::Battlefield,
        Zone::Graveyard,
        EventCause::from_effect(source, alice),
        &mut dm,
    );
    assert_eq!(nonmatching, EventOutcome::Proceed(Zone::Graveyard));
    assert_eq!(zombie_count(&game), 1, "nonmatching death created a token");
}

#[test]
fn exile_cycling_card_to_graveyard_replacement_matches_battlefield_zone_change() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardDefinitionBuilder::new(CardId::from_raw(4500), "Abandoned Probe")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(
                ObjectFilter::default().with_ability_marker("cycling"),
                PlayerFilter::You,
            ),
        ))
        .build();
    let source = game.create_object_from_definition(&source_card, alice, Zone::Battlefield);
    let cycling_card = CardDefinitionBuilder::new(CardId::from_raw(4501), "Cycling Creature Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}")
        .expect("cycling ability should parse");
    let cycling_object =
        game.create_object_from_definition(&cycling_card, alice, Zone::Battlefield);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = crate::events::processing::process_zone_change(
        &mut game,
        cycling_object,
        Zone::Battlefield,
        Zone::Graveyard,
        EventCause::from_cost(source, alice),
        &mut dm,
    );

    assert!(
        matches!(result, EventOutcome::Proceed(Zone::Exile)),
        "result={result:?}"
    );
}

#[test]
fn test_keyword_marker() {
    let ability = KeywordMarker::new("test marker");
    assert_eq!(ability.id(), StaticAbilityId::KeywordMarker);
    assert_eq!(ability.display(), "test marker");
}

#[test]
fn more_than_meets_the_eye_marker_has_canonical_keyword_casing_for_any_cost() {
    for (marker, expected) in [
        (
            "more than meets the eye {2}{u}{r}{w}",
            "More Than Meets the Eye {2}{u}{r}{w}",
        ),
        (
            "MORE THAN MEETS THE EYE {1}{r}{w}{b}",
            "More Than Meets the Eye {1}{r}{w}{b}",
        ),
    ] {
        assert_eq!(KeywordMarker::new(marker).display(), expected);
    }

    assert_eq!(
        KeywordMarker::new("more than meets the eyesight").display(),
        "more than meets the eyesight"
    );
}

#[test]
fn static_ability_keyword_marker_uses_semantic_marker() {
    let ability = StaticAbility::keyword_marker("test marker");
    assert_eq!(ability.id(), StaticAbilityId::KeywordMarker);
    assert_eq!(ability.display(), "test marker");
}

#[test]
fn test_keyword_text() {
    let ability = KeywordText::new("Dredge 3");
    assert_eq!(ability.id(), StaticAbilityId::KeywordText);
    assert_eq!(ability.display(), "Dredge 3");
}

#[test]
fn test_look_at_top_card_of_library() {
    let ability = LookAtTopCardOfLibrary;
    assert_eq!(ability.id(), StaticAbilityId::LookAtTopCardOfLibrary);
    assert_eq!(
        ability.display(),
        "You may look at the top card of your library any time."
    );
}

#[test]
fn test_look_at_face_down_creatures_you_dont_control() {
    let ability = LookAtFaceDownCreaturesYouDontControl;
    assert_eq!(
        ability.id(),
        StaticAbilityId::LookAtFaceDownCreaturesYouDontControl
    );
    assert_eq!(
        ability.display(),
        "You may look at face-down creatures you don't control any time."
    );
}

#[test]
fn test_all_players_look_at_top_cards_of_libraries() {
    let ability = AllPlayersLookAtTopCardsOfLibraries;
    assert_eq!(
        ability.id(),
        StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries
    );
    assert_eq!(
        ability.display(),
        "Players play with the top card of their libraries revealed."
    );
}

#[test]
fn test_all_players_look_at_your_top_library_card() {
    let ability = AllPlayersLookAtYourTopLibraryCard;
    assert_eq!(
        ability.id(),
        StaticAbilityId::AllPlayersLookAtYourTopLibraryCard
    );
    assert_eq!(
        ability.display(),
        "Play with the top card of your library revealed."
    );
}

#[test]
fn test_opponents_play_with_hands_revealed() {
    let ability = OpponentsPlayWithHandsRevealed;
    assert_eq!(
        ability.id(),
        StaticAbilityId::OpponentsPlayWithHandsRevealed
    );
    assert_eq!(
        ability.display(),
        "Your opponents play with their hands revealed."
    );
}

#[test]
fn test_unsupported_parser_line() {
    let ability = UnsupportedParserLine::new("Some unsupported line.", "ParseError(\"mock\")");
    assert_eq!(ability.id(), StaticAbilityId::UnsupportedParserLine);
    assert_eq!(
        ability.display(),
        "Unsupported parser line fallback: Some unsupported line. (ParseError(\"mock\"))"
    );
}

#[test]
fn test_morph_static_ability_reports_turn_face_up_cost() {
    let cost = crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
        crate::mana::ManaSymbol::Generic(3),
    ]]));
    let ability = Morph::new(cost.clone());
    assert_eq!(ability.id(), StaticAbilityId::Morph);
    assert_eq!(ability.turn_face_up_cost(), Some(&cost));
    assert!(!ability.is_megamorph());
    assert!(!ability.is_disguise());
}

#[test]
fn test_disguise_static_ability_reports_turn_face_up_cost() {
    let cost = crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
        crate::mana::ManaSymbol::Generic(2),
    ]]));
    let ability = Disguise::new(cost.clone());
    assert_eq!(ability.id(), StaticAbilityId::Disguise);
    assert_eq!(ability.turn_face_up_cost(), Some(&cost));
    assert!(ability.is_disguise());
    assert!(!ability.is_megamorph());
}

#[test]
fn test_megamorph_static_ability_reports_turn_face_up_cost() {
    let cost = crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
        crate::mana::ManaSymbol::Green,
    ]]));
    let ability = Megamorph::new(cost.clone());
    assert_eq!(ability.id(), StaticAbilityId::Megamorph);
    assert_eq!(ability.turn_face_up_cost(), Some(&cost));
    assert!(ability.is_megamorph());
}

#[test]
fn test_bloodthirst_replacement_matches_when_opponent_was_dealt_damage() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&damage_event);

    let ability = Bloodthirst::new(2);
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("bloodthirst should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("bloodthirst replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);

    assert!(
        matcher.matches_event(&event, &ctx),
        "bloodthirst should match when an opponent was dealt damage"
    );
}

#[test]
fn test_bloodthirst_replacement_does_not_match_without_opponent_damage() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);

    let ability = Bloodthirst::new(2);
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("bloodthirst should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("bloodthirst replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);

    assert!(
        !matcher.matches_event(&event, &ctx),
        "bloodthirst should not match when no opponent was dealt damage"
    );
}

#[test]
fn test_enters_with_counters_if_condition_matches_when_true() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(52);
    let alice = PlayerId::from_index(0);
    game.turn_store
        .turn_history
        .players_attacked_this_turn
        .insert(alice);

    let ability = EntersWithCountersIfCondition::new(
        CounterType::PlusOnePlusOne,
        Value::Fixed(1),
        Condition::AttackedThisTurn,
        "you attacked this turn".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-with-counters should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-with-counters replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);
    assert!(
        matcher.matches_event(&event, &ctx),
        "conditional enters-with-counters should match when condition is true"
    );
}

#[test]
fn test_enters_with_counters_if_condition_does_not_match_when_false() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(52);
    let alice = PlayerId::from_index(0);

    let ability = EntersWithCountersIfCondition::new(
        CounterType::PlusOnePlusOne,
        Value::Fixed(1),
        Condition::AttackedThisTurn,
        "you attacked this turn".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-with-counters should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-with-counters replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);
    assert!(
        !matcher.matches_event(&event, &ctx),
        "conditional enters-with-counters should not match when condition is false"
    );
}

#[test]
fn ardenvale_paladin_enters_with_counter_when_three_white_was_spent() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(70151), "Ardenvale Paladin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(5)))
        .build();

    let source = game.create_object_from_card(&card, alice, Zone::Stack);
    {
        let source_obj = game.object_mut(source).expect("source should exist");
        source_obj.abilities_mut().push(Ability::static_ability(StaticAbility::new(
                EntersWithCountersIfCondition::new(
                    CounterType::PlusOnePlusOne,
                    Value::Fixed(1),
                    Condition::ManaSpentToCastThisSpellAtLeast {
                        amount: 3,
                        symbol: Some(crate::mana::ManaSymbol::White),
                    },
                    "If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it".to_string(),
                ),
            )));
        let mut spent = crate::player::ManaPool::new();
        spent.add(crate::mana::ManaSymbol::White, 3);
        spent.add(crate::mana::ManaSymbol::Colorless, 1);
        source_obj.mana_spent_to_cast = spent;
    }

    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut decision_maker)
        .expect("Ardenvale Paladin should enter the battlefield");
    let permanent = game
        .object(result.new_id)
        .expect("Ardenvale Paladin should exist on battlefield");
    assert_eq!(
        permanent
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied(),
        Some(1)
    );
}

#[test]
fn ardenvale_paladin_does_not_get_counter_without_three_white_spent() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(70151), "Ardenvale Paladin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(5)))
        .build();

    let source = game.create_object_from_card(&card, alice, Zone::Stack);
    {
        let source_obj = game.object_mut(source).expect("source should exist");
        source_obj.abilities_mut().push(Ability::static_ability(StaticAbility::new(
                EntersWithCountersIfCondition::new(
                    CounterType::PlusOnePlusOne,
                    Value::Fixed(1),
                    Condition::ManaSpentToCastThisSpellAtLeast {
                        amount: 3,
                        symbol: Some(crate::mana::ManaSymbol::White),
                    },
                    "If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it".to_string(),
                ),
            )));
        let mut spent = crate::player::ManaPool::new();
        spent.add(crate::mana::ManaSymbol::White, 2);
        spent.add(crate::mana::ManaSymbol::Colorless, 2);
        source_obj.mana_spent_to_cast = spent;
    }

    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut decision_maker)
        .expect("Ardenvale Paladin should enter the battlefield");
    let permanent = game
        .object(result.new_id)
        .expect("Ardenvale Paladin should exist on battlefield");
    assert_eq!(
        permanent
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied(),
        None
    );
}

#[test]
fn enters_with_counters_if_kicked_uses_discarded_cost_card_mana_value() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let discarded_card = CardBuilder::new(CardId::from_raw(6100), "Discarded Baloth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![crate::mana::ManaSymbol::Generic(3)],
            vec![crate::mana::ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .build();
    let discarded_id = game.create_object_from_card(&discarded_card, alice, Zone::Graveyard);
    let discarded_snapshot = {
        let discarded = game
            .object(discarded_id)
            .expect("discarded card should exist");
        crate::snapshot::ObjectSnapshot::from_object(discarded, &game)
    };

    let source_card = CardBuilder::new(CardId::from_raw(6101), "Kicked Pet")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(2)))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let mut paid = crate::cost::OptionalCostsPaid::new(1);
    paid.mark_label_paid("Kicker");
    let enters = EntersWithCountersIfCondition::new_with_abilities(
        CounterType::PlusOnePlusOne,
        Value::ManaValueOf(Box::new(ChooseSpec::Tagged(crate::tag::TagKey::from(
            "discarded_cost",
        )))),
        Condition::ThisSpellWasKicked,
        "this creature was kicked".to_string(),
        vec![Ability::static_ability(StaticAbility::flying())],
    );
    {
        let source_obj = game.object_mut(source).expect("source should exist");
        source_obj.optional_costs_paid = paid;
        source_obj.cast_tagged_objects.insert(
            crate::tag::TagKey::from("discarded_cost"),
            vec![discarded_snapshot],
        );
        source_obj
            .abilities_mut()
            .push(Ability::static_ability(StaticAbility::new(enters)));
    }

    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut decision_maker)
        .expect("source should enter the battlefield");

    let permanent = game
        .object(result.new_id)
        .expect("permanent should exist on battlefield");
    assert_eq!(
        permanent
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied(),
        Some(4)
    );
    assert!(game.object_has_ability(result.new_id, &StaticAbility::flying()));
}

#[test]
fn test_enters_with_counters_if_condition_matches_when_opponent_lost_life() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(52);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::LifeLossEvent::from_effect(bob, 2),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);

    let ability = EntersWithCountersIfCondition::new(
        CounterType::PlusOnePlusOne,
        Value::Fixed(1),
        Condition::OpponentLostLifeThisTurn,
        "an opponent lost life this turn".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-with-counters should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-with-counters replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);
    assert!(
        matcher.matches_event(&event, &ctx),
        "conditional enters-with-counters should match when an opponent lost life this turn"
    );
}

#[test]
fn test_enters_with_counters_if_condition_matches_when_permanent_left_battlefield() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(52);
    let alice = PlayerId::from_index(0);
    let departed_permanent = CardBuilder::new(crate::ids::CardId::from_raw(9991), "Spent Relic")
        .card_types(vec![crate::types::CardType::Artifact])
        .build();
    let departed_id = game.create_object_from_card(&departed_permanent, alice, Zone::Battlefield);
    game.move_object_by_effect(departed_id, Zone::Graveyard);

    let ability = EntersWithCountersIfCondition::new(
        CounterType::PlusOnePlusOne,
        Value::Fixed(1),
        Condition::PermanentLeftBattlefieldUnderYourControlThisTurn {
            surface: crate::effect::PermanentLeftBattlefieldControlSurface::LeftUnderYourControl,
        },
        "a permanent left the battlefield under your control this turn".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-with-counters should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-with-counters replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);
    assert!(
        matcher.matches_event(&event, &ctx),
        "conditional enters-with-counters should match when a permanent left under your control"
    );
}

#[test]
fn test_enters_tapped_unless_first_three_turns_does_not_match_on_your_turn_three() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(77);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.turn_number = 3;

    let ability = EntersTappedUnlessCondition::new(
        Condition::YourFirstTurnsOfTheGameOrFewer(3),
        "it's your first second or third turn of the game".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-tapped replacement should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-tapped replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);

    assert!(
        !matcher.matches_event(&event, &ctx),
        "replacement should not apply during one of your first three turns"
    );
}

#[test]
fn test_enters_tapped_unless_first_three_turns_matches_on_your_turn_four() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(78);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.turn_number = 4;

    let ability = EntersTappedUnlessCondition::new(
        Condition::YourFirstTurnsOfTheGameOrFewer(3),
        "it's your first second or third turn of the game".to_string(),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("conditional enters-tapped replacement should create replacement");
    let matcher = replacement
        .matcher
        .as_ref()
        .expect("conditional enters-tapped replacement must have matcher");
    let event = ZoneChangeEvent::with_cause(
        source,
        Zone::Stack,
        Zone::Battlefield,
        crate::events::cause::EventCause::effect(),
        None,
    );
    let ctx = EventContext::for_replacement_effect(alice, source, &game);

    assert!(
        matcher.matches_event(&event, &ctx),
        "replacement should apply after your first three turns"
    );
}

#[test]
fn test_prevent_all_damage_dealt_by_this_permanent_generates_replacement() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let src = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);

    let ability = PreventAllDamageDealtByThisPermanent;
    let replacement = ability
        .generate_replacement_effect(src, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, src, &game);

    // Preventable damage from this permanent matches.
    let dmg = DamageEvent::with_cause(
        src,
        DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&dmg, &ctx));

    // Unpreventable damage from this permanent does not match.
    let unpreventable = DamageEvent::unpreventable_with_cause(
        src,
        DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&unpreventable, &ctx));
}

#[test]
fn test_prevent_all_combat_damage_dealt_by_this_permanent_generates_replacement() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let src = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);

    let ability = PreventAllCombatDamageDealtByThisPermanent;
    let replacement = ability
        .generate_replacement_effect(src, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, src, &game);

    let combat = DamageEvent::with_cause(
        src,
        DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&combat, &ctx));

    let noncombat = DamageEvent::with_cause(
        src,
        DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncombat, &ctx));

    let from_other = DamageEvent::with_cause(
        ObjectId::from_raw(7),
        DamageTarget::Player(alice),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&from_other, &ctx));
}

#[test]
fn test_prevent_all_damage_dealt_to_creatures_generates_replacement() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let src = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Creature Target")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

    let ability = PreventAllDamageDealtToCreatures;
    let replacement = ability
        .generate_replacement_effect(src, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, src, &game);

    let creature_damage = DamageEvent::with_cause(
        src,
        DamageTarget::Object(creature_id),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&creature_damage, &ctx));

    let player_damage = DamageEvent::with_cause(
        src,
        DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&player_damage, &ctx));
}

#[test]
fn test_prevent_all_damage_to_self_by_creatures_generates_replacement() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let protected_card = CardBuilder::new(CardId::new(), "Protected Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let protected = game.create_object_from_card(&protected_card, alice, Zone::Battlefield);

    let creature_source_card = CardBuilder::new(CardId::new(), "Creature Source")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_source =
        game.create_object_from_card(&creature_source_card, alice, Zone::Battlefield);

    let noncreature_source_card = CardBuilder::new(CardId::new(), "Artifact Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let noncreature_source =
        game.create_object_from_card(&noncreature_source_card, alice, Zone::Battlefield);

    let ability = PreventAllDamageToSelfByCreatures;
    let replacement = ability
        .generate_replacement_effect(protected, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, protected, &game);

    let creature_damage = DamageEvent::with_cause(
        creature_source,
        DamageTarget::Object(protected),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&creature_damage, &ctx));

    let noncreature_damage = DamageEvent::with_cause(
        noncreature_source,
        DamageTarget::Object(protected),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncreature_damage, &ctx));
}

#[test]
fn test_prevent_all_noncombat_damage_to_permanents_matching_generates_replacement() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = CardBuilder::new(CardId::new(), "Mark of Asylum Probe")
        .card_types(vec![CardType::Enchantment])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

    let protected = CardBuilder::new(CardId::new(), "Protected Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let protected_id = game.create_object_from_card(&protected, alice, Zone::Battlefield);

    let opponent = CardBuilder::new(CardId::new(), "Opponent Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let opponent_id = game.create_object_from_card(&opponent, bob, Zone::Battlefield);

    let damage_source = CardBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let damage_source_id = game.create_object_from_card(&damage_source, bob, Zone::Battlefield);

    let mut filter = ObjectFilter::creature();
    filter.controller = Some(PlayerFilter::You);
    let ability = PreventAllNoncombatDamageToPermanentsMatching::new(filter);
    let replacement = ability
        .generate_replacement_effect(source_id, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, source_id, &game);

    let noncombat_to_controlled_creature = DamageEvent::with_cause(
        damage_source_id,
        DamageTarget::Object(protected_id),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&noncombat_to_controlled_creature, &ctx));

    let combat_to_controlled_creature = DamageEvent::with_cause(
        damage_source_id,
        DamageTarget::Object(protected_id),
        2,
        true,
        crate::events::cause::EventCause::combat_damage(damage_source_id),
    );
    assert!(!matcher.matches_event(&combat_to_controlled_creature, &ctx));

    let noncombat_to_opponent_creature = DamageEvent::with_cause(
        damage_source_id,
        DamageTarget::Object(opponent_id),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncombat_to_opponent_creature, &ctx));

    let unpreventable = DamageEvent::unpreventable_with_cause(
        damage_source_id,
        DamageTarget::Object(protected_id),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&unpreventable, &ctx));
}

#[test]
fn test_prevent_all_combat_damage_to_self_generates_replacement() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let protected = ObjectId::from_raw(42);
    let source = ObjectId::from_raw(7);

    let ability = PreventAllCombatDamageToSelf;
    let replacement = ability
        .generate_replacement_effect(protected, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, protected, &game);

    let combat_damage = DamageEvent::with_cause(
        source,
        DamageTarget::Object(protected),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(source),
    );
    assert!(matcher.matches_event(&combat_damage, &ctx));

    let noncombat_damage = DamageEvent::with_cause(
        source,
        DamageTarget::Object(protected),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncombat_damage, &ctx));

    let unpreventable = DamageEvent::unpreventable_with_cause(
        source,
        DamageTarget::Object(protected),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(source),
    );
    assert!(!matcher.matches_event(&unpreventable, &ctx));
}

#[test]
fn test_prevent_all_damage_to_self_generates_replacement() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let protected = ObjectId::from_raw(42);
    let source = ObjectId::from_raw(7);

    let ability = PreventAllDamageToSelf;
    let replacement = ability
        .generate_replacement_effect(protected, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, protected, &game);

    let combat_damage = DamageEvent::with_cause(
        source,
        DamageTarget::Object(protected),
        3,
        true,
        crate::events::cause::EventCause::combat_damage(source),
    );
    assert!(matcher.matches_event(&combat_damage, &ctx));

    let noncombat_damage = DamageEvent::with_cause(
        source,
        DamageTarget::Object(protected),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&noncombat_damage, &ctx));

    let wrong_target = DamageEvent::with_cause(
        source,
        DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&wrong_target, &ctx));
}

#[test]
fn test_prevent_all_noncombat_damage_to_other_creatures_you_control() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source = CardBuilder::new(CardId::new(), "Crystal Barricade")
        .card_types(vec![CardType::Creature])
        .build();
    let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);

    let other = CardBuilder::new(CardId::new(), "Ally")
        .card_types(vec![CardType::Creature])
        .build();
    let other_id = game.create_object_from_card(&other, alice, Zone::Battlefield);

    let opponent = CardBuilder::new(CardId::new(), "Opponent Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let opponent_id = game.create_object_from_card(&opponent, bob, Zone::Battlefield);

    let ability = PreventAllNoncombatDamageToOtherCreaturesYouControl;
    let replacement = ability
        .generate_replacement_effect(source_id, alice)
        .expect("should generate replacement effect");
    assert_eq!(replacement.replacement, ReplacementAction::PreventDamage);

    let matcher = replacement
        .matcher
        .as_ref()
        .expect("replacement must have a matcher");
    let ctx = EventContext::for_replacement_effect(alice, source_id, &game);

    let noncombat_to_other = DamageEvent::with_cause(
        opponent_id,
        DamageTarget::Object(other_id),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(matcher.matches_event(&noncombat_to_other, &ctx));

    let combat_to_other = DamageEvent::with_cause(
        opponent_id,
        DamageTarget::Object(other_id),
        2,
        true,
        crate::events::cause::EventCause::combat_damage(opponent_id),
    );
    assert!(!matcher.matches_event(&combat_to_other, &ctx));

    let noncombat_to_source = DamageEvent::with_cause(
        opponent_id,
        DamageTarget::Object(source_id),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert!(!matcher.matches_event(&noncombat_to_source, &ctx));
}

#[test]
fn test_prevent_damage_to_self_remove_counter_generates_replacement() {
    let src = ObjectId::from_raw(42);
    let alice = PlayerId::from_index(0);

    let ability = PreventDamageToSelfRemoveCounter::new(CounterType::PlusOnePlusOne, 1);
    let replacement = ability
        .generate_replacement_effect(src, alice)
        .expect("should generate replacement effect");

    let ReplacementAction::Instead(effects) = &replacement.replacement else {
        panic!("expected replacement to use Instead action");
    };
    assert_eq!(effects.len(), 1, "expected one removal effect");
    let remove = effects[0]
        .downcast_ref::<crate::effects::RemoveCountersEffect>()
        .expect("expected remove counters effect");
    assert_eq!(remove.counter_type, CounterType::PlusOnePlusOne);
    assert_eq!(remove.count, Value::Fixed(1));
    assert!(matches!(remove.target, ChooseSpec::Source));

    let dynamic = PreventDamageToSelfRemoveCounter::new(
        CounterType::PlusOnePlusOne,
        Value::EventValue(EventValueSpec::Amount),
    );
    let replacement = dynamic
        .generate_replacement_effect(src, alice)
        .expect("dynamic prevention should generate replacement effect");
    let ReplacementAction::Instead(effects) = &replacement.replacement else {
        panic!("expected dynamic replacement to use Instead action");
    };
    let remove = effects[0]
        .downcast_ref::<crate::effects::RemoveCountersEffect>()
        .expect("expected dynamic remove counters effect");
    assert_eq!(remove.count, Value::EventValue(EventValueSpec::Amount));

    let per_damage =
        PreventDamageToSelfRemoveCounter::new_one_damage_per_counter(CounterType::PlusOnePlusOne);
    let replacement = per_damage
        .generate_replacement_effect(src, alice)
        .expect("per-damage prevention should generate a replacement effect");
    assert!(matches!(
        replacement.replacement,
        ReplacementAction::PreventDamageByRemovingSourceCounters {
            counter_type: CounterType::PlusOnePlusOne,
        }
    ));
}

#[test]
fn separate_sentence_surface_keeps_counter_removal_prevention_executable() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let phantom_card = CardBuilder::new(CardId::new(), "Phantom Surface Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let damage_source_card = CardBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let phantom = game.create_object_from_card(&phantom_card, alice, Zone::Battlefield);
    let damage_source = game.create_object_from_card(&damage_source_card, bob, Zone::Battlefield);
    game.add_counters(phantom, CounterType::PlusOnePlusOne, 2);

    let ability = PreventDamageToSelfRemoveCounter::new(CounterType::PlusOnePlusOne, 1)
        .with_surface(ironsmith_core::CounterRemovalPreventionSurface::SeparateSentences);
    assert_eq!(
        ability.display(),
        "If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature."
    );
    game.effect_store.replacement_effects.add_resolution_effect(
        ability
            .generate_replacement_effect(phantom, alice)
            .expect("separate-sentence prevention should generate a replacement"),
    );

    let result = process_damage_assignments_with_event(
        &mut game,
        damage_source,
        DamageTarget::Object(phantom),
        5,
        false,
        EventCause::from_effect(damage_source, bob),
    );

    assert!(result.replacement_prevented);
    assert!(result.assignments.is_empty());
    assert_eq!(game.counter_count(phantom, CounterType::PlusOnePlusOne), 1);
}

#[test]
fn counter_prevention_followup_uses_actual_removed_count_for_each_player() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_card = CardBuilder::new(CardId::new(), "Counter Shield")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    game.add_counters(source, CounterType::PlusOnePlusOne, 2);

    let ability = PreventDamageToSelfRemoveCounter::new_with_follow_up(
        CounterType::PlusOnePlusOne,
        Value::EventValue(EventValueSpec::Amount),
        Some(
            ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
                counter_type: CounterType::Rad,
                counters_per_removed: 1,
            },
        ),
    );
    let replacement = ability
        .generate_replacement_effect(source, alice)
        .expect("prevention should generate a replacement");
    let ReplacementAction::Instead(effects) = replacement.replacement else {
        panic!("expected replacement effects");
    };
    assert_eq!(effects.len(), 2);

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(
            ObjectId::from_raw(99),
            DamageTarget::Object(source),
            5,
            false,
            EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_triggering_event(damage_event);
    for effect in effects {
        crate::effects::execute_effect(&mut game, &effect, &mut ctx)
            .expect("replacement follow-up should resolve");
    }

    assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
    assert_eq!(
        game.player(alice)
            .expect("alice exists")
            .counter_count(CounterType::Rad),
        2
    );
    assert_eq!(
        game.player(bob)
            .expect("bob exists")
            .counter_count(CounterType::Rad),
        2
    );
}

#[test]
fn test_umbra_armor_replaces_destroy_effect() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let creature = CardBuilder::new(CardId::new(), "Protected Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    game.mark_damage(creature_id, 2);

    let mut aura_def = crate::cards::CardDefinition::new(
        CardBuilder::new(CardId::new(), "Umbra Shell")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build(),
    );
    aura_def
        .abilities
        .push(crate::ability::Ability::static_ability(
            StaticAbility::umbra_armor(),
        ));
    let aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    game.object_mut(aura_id).expect("aura exists").attached_to =
        Some(crate::object::AttachmentTarget::Object(creature_id));
    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(aura_id);

    let result = crate::events::processing::process_destroy_full(&mut game, creature_id, None);
    assert_eq!(result, crate::events::processing::DestroyResult::Replaced);
    assert!(
        game.object(creature_id).is_some(),
        "creature should survive"
    );
    assert_eq!(
        game.damage_on(creature_id),
        0,
        "umbra armor should clear marked damage"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Umbra Shell"),
        true,
        "umbra armor aura should be destroyed"
    );
}

#[test]
fn test_umbra_armor_replaces_lethal_damage_state_based_destruction() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let creature = CardBuilder::new(CardId::new(), "Protected Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    game.mark_damage(creature_id, 2);

    let mut aura_def = crate::cards::CardDefinition::new(
        CardBuilder::new(CardId::new(), "Umbra Shell")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build(),
    );
    aura_def
        .abilities
        .push(crate::ability::Ability::static_ability(
            StaticAbility::umbra_armor(),
        ));
    let aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    game.object_mut(aura_id).expect("aura exists").attached_to =
        Some(crate::object::AttachmentTarget::Object(creature_id));
    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(aura_id);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    assert!(
        apply_state_based_actions_with(&mut game, &mut dm),
        "lethal damage should apply a state-based action"
    );
    assert!(
        game.object(creature_id).is_some(),
        "creature should survive"
    );
    assert_eq!(game.damage_on(creature_id), 0);
    assert!(
        game.player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|obj| obj.name == "Umbra Shell"),
        "umbra armor aura should be in the graveyard after replacing lethal damage"
    );
}

fn creatures_you_control_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    filter.controller = Some(PlayerFilter::You);
    filter
}

fn entering_mana_value_four_or_less() -> Condition {
    Condition::ValueComparison {
        left: Value::ManaValueOf(Box::new(ChooseSpec::Source)),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Fixed(4),
    }
}

#[test]
fn filtered_enters_counters_if_otherwise_renders_and_uses_entering_mana_value() {
    let mut subject = creatures_you_control_filter();
    subject.other = true;
    let ability =
        EnterWithCountersForFilter::new(subject, CounterType::PlusOnePlusOne, Value::Fixed(1))
            .with_count_if_otherwise(entering_mana_value_four_or_less(), Value::Fixed(3));
    assert_eq!(
        ability.display(),
        "Each other creature you control enters with an additional +1/+1 counter on it if its mana value is 4 or less. Otherwise, it enters with three additional +1/+1 counters on it"
    );

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::from_raw(6210), "Counter Branch Source")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::new(ability)))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let creature_with_mana_value = |id, name, mana_value| {
        CardDefinitionBuilder::new(CardId::from_raw(id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(mana_value),
            ]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(2)))
            .build()
    };
    let low = creature_with_mana_value(6211, "Low Mana Creature", 4);
    let high = creature_with_mana_value(6212, "High Mana Creature", 5);
    let low_id = game.create_object_from_definition(&low, alice, Zone::Stack);
    let high_id = game.create_object_from_definition(&high, alice, Zone::Stack);
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let low_result = game
        .move_object_with_etb_processing_with_dm(low_id, Zone::Battlefield, &mut decision_maker)
        .expect("low-mana creature should enter");
    let high_result = game
        .move_object_with_etb_processing_with_dm(high_id, Zone::Battlefield, &mut decision_maker)
        .expect("high-mana creature should enter");

    assert_eq!(
        game.object(low_result.new_id)
            .and_then(|object| object.counters.get(&CounterType::PlusOnePlusOne))
            .copied(),
        Some(1)
    );
    assert_eq!(
        game.object(high_result.new_id)
            .and_then(|object| object.counters.get(&CounterType::PlusOnePlusOne))
            .copied(),
        Some(3)
    );
}

#[test]
fn filtered_enters_counters_render_for_each_land_entered_this_turn() {
    let mut lands = ObjectFilter::land();
    lands.controller = Some(PlayerFilter::You);
    let count =
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::EnteredBattlefield(lands))
            .with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EnterWithCountersForFilter::new(
        creatures_you_control_filter(),
        CounterType::PlusOnePlusOne,
        count,
    );

    assert_eq!(
        ability.display(),
        "Each creature you control enters with an additional +1/+1 counter on it for each land that entered the battlefield under your control this turn"
    );
}

#[test]
fn filtered_enters_counters_keep_each_other_subject_and_lost_life_basis() {
    let mut subject = creatures_you_control_filter();
    subject.other = true;
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersLostLife(
        PlayerFilter::Opponent,
    ))
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EnterWithCountersForFilter::new(subject, CounterType::PlusOnePlusOne, count);

    assert_eq!(
        ability.display(),
        "Each other creature you control enters with an additional +1/+1 counter on it for each opponent who lost life this turn"
    );
}

#[test]
fn filtered_enters_counters_render_matching_object_count_as_for_each() {
    let mut dog = ObjectFilter::default();
    dog.subtypes = vec![Subtype::Dog];
    let mut wolf = ObjectFilter::default();
    wolf.subtypes = vec![Subtype::Wolf];
    let mut dogs_or_wolves = ObjectFilter::default()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    dogs_or_wolves.any_of = vec![dog, wolf];
    let count = Value::Count(dogs_or_wolves).with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EnterWithCountersForFilter::new(
        creatures_you_control_filter(),
        CounterType::PlusOnePlusOne,
        count,
    );

    assert_eq!(
        ability.display(),
        "Each creature you control enters with an additional +1/+1 counter on it for each Dog or Wolf you control"
    );
}

#[test]
fn filtered_enters_counters_render_mana_source_provenance_basis() {
    let coin_count = Value::ManaFromSourceSpentToCastThisSpell {
        source_filter: ObjectFilter::artifact(),
        include_source_noun: true,
        reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let coin = EnterWithCountersForFilter::new(
        creatures_you_control_filter(),
        CounterType::PlusOnePlusOne,
        coin_count,
    );
    assert_eq!(
        coin.display(),
        "Each creature you control enters with an additional +1/+1 counter on it for each mana from an artifact source spent to cast it"
    );

    let mut other_creatures = creatures_you_control_filter();
    other_creatures.other = true;
    let kalain_count = Value::ManaFromSourceSpentToCastThisSpell {
        source_filter: ObjectFilter::artifact().with_subtype(Subtype::Treasure),
        include_source_noun: false,
        reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let kalain =
        EnterWithCountersForFilter::new(other_creatures, CounterType::PlusOnePlusOne, kalain_count);
    assert_eq!(
        kalain.display(),
        "Each other creature you control enters with an additional +1/+1 counter on it for each mana from a Treasure spent to cast it"
    );
}

#[test]
fn filtered_enters_counters_count_matching_mana_source_snapshots_on_entering_spell() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let count = Value::ManaFromSourceSpentToCastThisSpell {
        source_filter: ObjectFilter::artifact(),
        include_source_noun: true,
        reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
    }
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let coin = CardDefinitionBuilder::new(CardId::from_raw(6200), "Coin")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::new(
            EnterWithCountersForFilter::new(
                creatures_you_control_filter(),
                CounterType::PlusOnePlusOne,
                count,
            ),
        )))
        .build();
    game.create_object_from_definition(&coin, alice, Zone::Battlefield);

    let artifact = CardDefinitionBuilder::new(CardId::from_raw(6201), "Mana Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let artifact_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(artifact_id)
            .expect("artifact source should exist"),
        &game,
    );
    let land = CardDefinitionBuilder::new(CardId::from_raw(6202), "Mana Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let land_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(land_id).expect("land source should exist"),
        &game,
    );

    let creature = CardDefinitionBuilder::new(CardId::from_raw(6203), "Coin Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(2), PtValue::Fixed(2)))
        .build();
    let creature_id = game.create_object_from_definition(&creature, alice, Zone::Stack);
    game.object_mut(creature_id)
        .expect("creature spell should exist")
        .cast_tagged_objects
        .insert(
            crate::tag::TagKey::from(ironsmith_core::MANA_SOURCES_SPENT_TO_CAST_TAG),
            vec![artifact_snapshot.clone(), artifact_snapshot, land_snapshot],
        );

    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(
            creature_id,
            Zone::Battlefield,
            &mut decision_maker,
        )
        .expect("creature should enter");
    assert_eq!(
        game.object(result.new_id)
            .expect("creature should be on the battlefield")
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied(),
        Some(2),
        "only the two artifact-produced mana units should add counters"
    );
}

#[test]
fn filtered_enters_counters_pluralize_nontoken_subject_and_died_basis() {
    let mut subject = creatures_you_control_filter();
    subject.nontoken = true;
    let mut creatures = ObjectFilter::creature();
    creatures.controller = Some(PlayerFilter::You);
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::died(creatures))
        .with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EnterWithCountersForFilter::new(subject, CounterType::PlusOnePlusOne, count);

    assert_eq!(
        ability.display(),
        "Nontoken creatures you control enter with an additional +1/+1 counter on them for each creature that died under your control this turn"
    );
}

#[test]
fn filtered_enters_counters_scale_repeated_turn_history_basis() {
    let history = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::PlayersLostLife(
        PlayerFilter::Opponent,
    ));
    let count = Value::Add(Box::new(history.clone()), Box::new(history))
        .with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EnterWithCountersForFilter::new(
        creatures_you_control_filter(),
        CounterType::PlusOnePlusOne,
        count,
    );

    assert_eq!(
        ability.display(),
        "Each creature you control enters with two additional +1/+1 counters on it for each opponent who lost life this turn"
    );
}

#[test]
fn enters_counters_render_for_each_other_spell_cast_this_turn() {
    let mut spells = ObjectFilter::default();
    spells.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let count = Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
        player: PlayerFilter::Any,
        filter: spells,
        from_zone: None,
        from_outside_hand: false,
        exclude_source: true,
        before_triggering_spell: false,
    })
    .with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EntersWithCounters::new(CounterType::PlusOnePlusOne, count);

    assert_eq!(
        ability.display(),
        "Enters the battlefield with a +1/+1 counter on it for each other spell cast this turn"
    );
}

#[test]
fn enters_counters_render_singular_revealed_card_for_each_basis() {
    let mut revealed = ObjectFilter::default();
    revealed.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("__public_revealed"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let count = Value::Count(revealed).with_surface_hint(ValueSurfaceHint::ForEach);
    let ability = EntersWithCounters::new(CounterType::PlusOnePlusOne, count);

    assert_eq!(
        ability.display(),
        "Enters the battlefield with a +1/+1 counter on it for each card revealed this way"
    );
}

#[test]
fn enters_counters_preserve_authored_additional_surface_hint() {
    let count = Value::Fixed(1).with_surface_hint(ValueSurfaceHint::AdditionalEntryCounter);
    let ability = EntersWithCounters::new(CounterType::PlusOnePlusOne, count);

    assert_eq!(
        ability.display(),
        "Enters the battlefield with an additional +1/+1 counter on it"
    );
}

#[test]
fn self_enters_with_dynamic_count_resolves_matching_permanents() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let island = CardBuilder::new(CardId::from_raw(62_100), "Island Probe")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .build();
    let plains = CardBuilder::new(CardId::from_raw(62_101), "Plains Probe")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build();
    game.create_object_from_card(&island, alice, Zone::Battlefield);
    game.create_object_from_card(&island, alice, Zone::Battlefield);
    game.create_object_from_card(&plains, alice, Zone::Battlefield);

    let source_card = CardBuilder::new(CardId::from_raw(62_102), "Dynamic Counter Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Fixed(1), PtValue::Fixed(1)))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let island_count = Value::Count(
        ObjectFilter::default()
            .with_subtype(Subtype::Island)
            .controlled_by(PlayerFilter::You)
            .in_zone(Zone::Battlefield),
    )
    .with_surface_hint(ValueSurfaceHint::ForEach);
    game.object_mut(source)
        .expect("source should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::enters_with_counters_value(CounterType::Time, island_count),
        ));

    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut decision_maker)
        .expect("source should enter");
    assert_eq!(
        game.object(result.new_id)
            .expect("source should be on the battlefield")
            .counters
            .get(&CounterType::Time)
            .copied(),
        Some(2)
    );
}

#[test]
fn play_permission_renders_narrow_cast_this_way_entry_counter_filter() {
    let count = Value::Fixed(1).with_surface_hint(ValueSurfaceHint::AdditionalEntryCounter);
    let grant = crate::grant::GrantSpec::new(
        crate::grant::Grantable::PlayFrom,
        ObjectFilter::default(),
        Zone::Library,
    )
    .with_cast_this_way_filter(ObjectFilter::creature())
    .with_cast_this_way_grant(StaticAbility::enters_with_counters_value(
        CounterType::PlusOnePlusOne,
        count,
    ));

    assert_eq!(
        grant.display(),
        "You may play lands and cast spells from the top of your library. If you cast a creature spell this way, that creature enters with an additional +1/+1 counter on it"
    );
}

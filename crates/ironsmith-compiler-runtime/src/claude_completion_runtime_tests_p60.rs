//! Claude package 60 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn hawkeye_labeled_modal_trigger_keeps_its_self_subject_and_bounded_payments() {
    let definition = compile_to_runtime_definition("Hawkeye, Master Marksman",
        "Mana cost: {1}{R}\nType: Legendary Creature — Human Archer Hero\nPower/Toughness: 2/2\nReach, first strike\nTrick Arrows — Whenever Hawkeye becomes tapped, you may pay {1} up to three times. When you do, choose up to that many —\n• Net — Target creature can't block this turn.\n• Explosive — Hawkeye deals 2 damage to target player.\n• Boomerang — Discard a card, then draw a card.", false)
        .expect("Hawkeye should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("matcher: BecomesTappedTrigger"),
        "the ability-word label must not leak 'whenever' into the trigger subject: {debug}");
    assert!(!debug.contains("PermanentBecomesTappedTrigger"), "{debug}");
    assert!(debug.contains("max_iterations: Some(3)"), "{debug}");
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("max: EffectValue(EffectId(1000000000))"), "the mode count is the payment count: {debug}");
}

#[test]
fn imodane_triggers_only_on_damage_to_the_spells_single_creature_target() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::events::{DamageEvent, DamageTarget, EventCause};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Imodane, the Pyrohammer",
        "Mana cost: {2}{R}{R}\nType: Legendary Creature — Human Knight\nPower/Toughness: 4/4\nWhenever an instant or sorcery spell you control that targets only a single creature deals damage to that creature, Imodane deals that much damage to each opponent.", false)
        .expect("Imodane should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("recipient_is_source_target: true"), "{debug}");
    assert!(debug.contains("targets_only_object: Some"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let imodane = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = |game: &mut ironsmith::GameState, name: &str| {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, bob, Zone::Battlefield)
    };
    let targeted = creature(&mut game, "Targeted creature");
    let bystander = creature(&mut game, "Bystander creature");
    let sorcery = CardBuilder::new(CardId::new(), "Witness sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell = game.create_object_from_card(&sorcery, alice, Zone::Stack);
    // The resolving spell's entry has been popped; its cast record keeps it.
    let spell_object = game.object(spell).unwrap().clone();
    let entry = ironsmith::game_state::StackEntry::new(spell, alice)
        .with_targets(vec![ironsmith::game_state::Target::Object(targeted)]);
    game.turn_store.cast_spell_lki.insert(spell, std::sync::Arc::new((spell_object, entry)));
    game.refresh_continuous_state().unwrap();
    let damage = |recipient| {
        let mut cause = EventCause::from_effect(spell, alice);
        cause.targeted_objects = vec![targeted];
        ironsmith::triggers::TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(spell, DamageTarget::Object(recipient), 3, false, cause),
            Default::default())
    };
    let entries = ironsmith::triggers::check_triggers(&game, &damage(targeted));
    assert!(entries.iter().any(|entry| entry.source == imodane),
        "damage to the spell's single creature target triggers");
    let entries = ironsmith::triggers::check_triggers(&game, &damage(bystander));
    assert!(entries.iter().all(|entry| entry.source != imodane),
        "damage to a creature the spell doesn't target doesn't trigger");
}

#[test]
fn havengul_lich_casts_from_any_graveyard_and_watches_the_cast_by_stable_identity() {
    let definition = compile_to_runtime_definition("Havengul Lich",
        "Mana cost: {3}{U}{B}\nType: Creature — Zombie Wizard\nPower/Toughness: 4/4\n{1}: You may cast target creature card in a graveyard this turn. When you cast it this turn, this creature gains all activated abilities of that card until end of turn.", false)
        .expect("Havengul Lich should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("zone: Some(Graveyard)"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("SpellCast"), "{debug}");
    assert!(debug.contains("relation: SameStableId"),
        "the watcher follows the card onto the stack: {debug}");
    assert!(!debug.contains("tag: TagKey(\"__it__\"), relation: SameStableId"),
        "the pronoun is bound to the targeted card's tag: {debug}");
}

#[test]
fn hogaak_cant_spend_mana_restriction_refuses_every_mana_unit() {
    let definition = compile_to_runtime_definition("Hogaak, Arisen Necropolis",
        "Mana cost: {5}{B/G}{B/G}\nType: Legendary Creature — Avatar\nPower/Toughness: 8/8\nYou can't spend mana to cast this spell.\nConvoke, delve (Each creature you tap while casting this spell pays for {1} or one mana of that creature's color. Each card you exile from your graveyard pays for {1}.)\nYou may cast this card from your graveyard.\nTrample", false)
        .expect("Hogaak should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SpellManaSpendingRestriction(NoMana)"), "{debug}");

    use ironsmith::ids::PlayerId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::zone::Zone;
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 3);
    let request = |cost: ManaCost| ironsmith::mana_payment::ManaPaymentRequest::new(
        alice, source, ironsmith::costs::PaymentReason::Effect, cost);
    let plain = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]);
    assert!(ironsmith::mana_payment::plan_mana_payment(&game, &request(plain.clone())).is_ok(),
        "floating mana pays an unrestricted cost");
    let restricted = plain.with_spending_restriction(ironsmith_core::mana::ManaSpendingRestriction::NoMana);
    assert!(ironsmith::mana_payment::plan_mana_payment(&game, &request(restricted)).is_err(),
        "no mana unit may be spent on a no-mana cost");
}

#[test]
fn illusionary_terrain_turns_basic_lands_of_the_first_chosen_type_into_the_second() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::{CardType, Subtype, Supertype};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Illusionary Terrain",
        "Mana cost: {U}{U}\nType: Enchantment\nCumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)\nAs this enchantment enters, choose two basic land types.\nBasic lands of the first chosen type are the second chosen type.", false)
        .expect("Illusionary Terrain should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ChooseBasicLandTypesAsEnters") && debug.contains("count: 2"), "{debug}");
    assert!(debug.contains("FirstChosenLandTypeBecomesSecond"), "{debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let terrain = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_basic_land_type_sequence(terrain, vec![Subtype::Island, Subtype::Mountain]);
    let land = |game: &mut ironsmith::GameState, name: &str, basic: bool| {
        let mut builder = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island]);
        if basic {
            builder = builder.supertypes(vec![Supertype::Basic]);
        }
        game.create_object_from_card(&builder.build(), alice, Zone::Battlefield)
    };
    let basic_island = land(&mut game, "Basic Island", true);
    let nonbasic_island = land(&mut game, "Nonbasic Island", false);
    game.refresh_continuous_state().unwrap();
    let subtypes = |game: &ironsmith::GameState, id| {
        game.calculated_characteristics(id).unwrap().subtypes.iter().copied().collect::<Vec<Subtype>>()
    };
    assert_eq!(subtypes(&game, basic_island), vec![Subtype::Mountain],
        "the basic Island is a Mountain instead (CR 305.7)");
    assert_eq!(subtypes(&game, nonbasic_island), vec![Subtype::Island],
        "only basic lands of the first chosen type change");
}

#[test]
fn helm_of_obedience_mills_until_a_creature_and_reanimates_it() {
    let definition = compile_to_runtime_definition("Helm of Obedience",
        "Mana cost: {4}\nType: Artifact\n{X}, {T}: Target opponent mills a card, then repeats this process until a creature card or X cards have been put into their graveyard this way, whichever comes first. If one or more creature cards were put into that graveyard this way, sacrifice this artifact and put one of them onto the battlefield under your control. X can't be 0.", false)
        .expect("Helm of Obedience should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("ConsultTopOfLibraryEffect") && debug.contains("mode: Mill"), "{debug}");
    assert!(debug.contains("max_exposed: Some(X)"), "{debug}");
    assert!(debug.contains("PutIntoGraveyard"), "{debug}");
    assert!(debug.contains("SacrificeTargetEffect"), "{debug}");
    assert!(!debug.contains("WithCount(Source"),
        "\"one of them\" names the milled creature, not the sacrificed artifact: {debug}");
}

#[test]
fn mill_consult_stops_at_the_first_creature_or_its_cap() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let setup = || {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        // Bottom to top: creature under two lands, so the top two are lands.
        let creature = CardBuilder::new(CardId::new(), "Milled creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&creature, bob, Zone::Library);
        for name in ["Milled land A", "Milled land B"] {
            let land = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Land]).build();
            game.create_object_from_card(&land, bob, Zone::Library);
        }
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Helm witness").card_types(vec![CardType::Artifact]).build(),
            alice, Zone::Battlefield);
        (game, source)
    };
    let consult = |max: i32| ironsmith::effect::Effect::new(
        ironsmith::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::Specific(bob),
            ironsmith::effects::consult_helpers::LibraryConsultMode::Mill,
            {
                let mut creature_card = ObjectFilter::creature();
                creature_card.zone = None;
                creature_card
            },
            ironsmith::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            "p60_milled",
            "p60_match",
        )
        .with_max_exposed(ironsmith::effect::Value::Fixed(max)),
    );
    let (mut game, source) = setup();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &consult(5), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 3, "two lands, then the creature");
    assert!(game.player(bob).unwrap().library.is_empty());

    let (mut game, source) = setup();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &consult(2), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(bob).unwrap().graveyard.len(), 2, "X cards came first");
    assert_eq!(game.player(bob).unwrap().library.len(), 1);
}

#[test]
fn heirloom_epic_activation_cost_can_be_paid_by_tapping_creatures_only() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Heirloom Epic",
        "Mana cost: {1}\nType: Artifact — Book\n{4}, {T}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana. Activate only as a sorcery.", false)
        .expect("Heirloom Epic should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("creatures_only: true"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
    assert!(!debug.contains("ForEach"), "the rider is a payment option, not an instruction: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let epic = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = |game: &mut ironsmith::GameState, name: &str| {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&card, alice, Zone::Battlefield)
    };
    for name in ["Tapper A", "Tapper B", "Tapper C"] {
        creature(&mut game, name);
    }
    let artifact = CardBuilder::new(CardId::new(), "Bystander artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    let cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(4)]).with_creature_tap_payment();
    let request = ironsmith::mana_payment::ManaPaymentRequest::new(
        alice, epic, ironsmith::costs::PaymentReason::ActivateAbility, cost);
    assert!(ironsmith::mana_payment::plan_mana_payment(&game, &request).is_err(),
        "three creatures cannot pay {{4}}, and the artifact is not eligible");
    creature(&mut game, "Tapper D");
    assert!(ironsmith::mana_payment::plan_mana_payment(&game, &request).is_ok(),
        "four untapped creatures pay the whole {{4}}");
}

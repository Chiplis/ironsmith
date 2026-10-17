//! Claude package 49 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn flash_conscription_conditionally_grants_a_quoted_life_gain_trigger() {
    // A "life" inside the granted quote belongs to the granted ability; it
    // must not disqualify the outer "the creature gains \"...\"" grant.
    let definition = compile_to_runtime_definition(
        "Flash Conscription",
        "Mana cost: {5}{R}\nType: Instant\nUntap target creature and gain control of it until end of turn. That creature gains haste until end of turn. If {W} was spent to cast this spell, the creature gains \"Whenever this creature deals combat damage, you gain that much life\" until end of turn.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("ThisDealsDamageTrigger"), "{debug}");
    assert!(debug.contains("combat_only: true"), "{debug}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
    assert!(debug.contains("EventValue(Amount)"), "{debug}");
    assert!(debug.contains("Haste"), "{debug}");
}

#[test]
fn gale_waterdeep_prodigy_casts_up_to_one_card_of_the_other_type_and_exiles_it() {
    let definition = compile_to_runtime_definition(
        "Gale, Waterdeep Prodigy",
        "Mana cost: {2}{U}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 1/3\nWhenever you cast an instant or sorcery spell from your hand, you may cast up to one target card of the other type from your graveyard. If a spell cast from your graveyard this way would be put into your graveyard, exile it instead.\nChoose a Background (You can have a Background as a second commander.)",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // "of the other type": an instant or sorcery card sharing no card type
    // with the triggering spell.
    assert!(debug.contains("SharesNoCardTypeWithTagged"), "{debug}");
    assert!(debug.contains("TagKey(\"triggering\")"), "{debug}");
    assert!(debug.contains("Instant") && debug.contains("Sorcery"), "{debug}");
    // "up to one target": the single target is optional.
    assert!(debug.contains("min: 0, max: Some(1)"), "{debug}");
    // The cast spell is exiled instead of going to the graveyard.
    assert!(debug.contains("Exile"), "{debug}");
    assert!(debug.contains("CastTagged") || debug.contains("cast_tagged"), "{debug}");
}

#[test]
fn gauntlets_of_chaos_exchanges_a_type_listed_target_and_destroys_their_auras() {
    let definition = compile_to_runtime_definition(
        "Gauntlets of Chaos",
        "Mana cost: {5}\nType: Artifact\n{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you control and target permanent an opponent controls that shares one of those types with it. If those permanents are exchanged this way, destroy all Auras attached to them.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // The comma-listed card types stay inside the first target phrase.
    assert!(debug.contains("ExchangeControlEffect"), "{debug}");
    assert!(debug.contains("shared_type: Some(CardType)"), "{debug}");
    assert!(debug.contains("Land"), "{debug}");
    // The aura sweep is gated on the exchange having happened.
    assert!(debug.contains("Happened"), "{debug}");
    assert!(debug.contains("DestroyEffect"), "{debug}");
    assert!(debug.contains("Aura"), "{debug}");
}

#[test]
fn frenzied_saddlebrute_lets_sick_creatures_attack_only_its_controllers_opponents() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition(
        "Frenzied Saddlebrute",
        "Mana cost: {4}{R}\nType: Creature — Orc Warrior\nPower/Toughness: 5/4\nHaste\nAll creatures can attack your opponents and planeswalkers your opponents control as though those creatures had haste.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("CreaturesCanAttackOpponentsAsThoughHaste"), "{debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let vanilla = CardBuilder::new(CardId::new(), "Sick fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut without = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let lone = without.create_object_from_card(&vanilla, alice, Zone::Battlefield);
    without.set_summoning_sick(lone);
    assert!(!ironsmith::rules::combat::can_attack_defending_player(
        without.object(lone).unwrap(),
        bob,
        &without
    ));

    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mine = game.create_object_from_card(&vanilla, alice, Zone::Battlefield);
    game.set_summoning_sick(mine);
    let theirs = game.create_object_from_card(&vanilla, bob, Zone::Battlefield);
    game.set_summoning_sick(theirs);
    // Alice's creature attacks her opponent as though it had haste.
    assert!(ironsmith::rules::combat::can_attack_defending_player(
        game.object(mine).unwrap(),
        bob,
        &game
    ));
    // Bob's creature would attack Alice, who is not an opponent of the
    // Saddlebrute's controller: still summoning sick.
    assert!(!ironsmith::rules::combat::can_attack_defending_player(
        game.object(theirs).unwrap(),
        alice,
        &game
    ));
}

#[test]
fn giant_albatross_iterates_creatures_that_damaged_it_with_per_creature_life_payment() {
    let definition = compile_to_runtime_definition(
        "Giant Albatross",
        "Mana cost: {1}{U}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying\nWhen this creature dies, you may pay {1}{U}. If you do, for each creature that dealt damage to this creature this turn, destroy that creature unless its controller pays 2 life. A creature destroyed this way can't be regenerated.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // The iterated set is the creatures that damaged the source this turn,
    // read against the source's (last-known) object.
    assert!(debug.contains("DealtDamageToTaggedThisTurn { combat_only: false }"), "{debug}");
    assert!(debug.contains("__source_object__"), "{debug}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    // Each creature's controller may pay 2 life to save it; otherwise it is
    // destroyed without regeneration.
    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
    assert!(debug.contains("DestroyNoRegenerationEffect"), "{debug}");
}

#[test]
fn frantic_scapegoat_passes_suspicion_to_one_creature_of_the_entry_batch() {
    let definition = compile_to_runtime_definition(
        "Frantic Scapegoat",
        "Mana cost: {R}\nType: Creature — Goat\nPower/Toughness: 1/1\nHaste\nWhen this creature enters, suspect it. (It has menace and can't block.)\nWhenever one or more other creatures you control enter, if this creature is suspected, you may suspect one of the other creatures. If you do, this creature is no longer suspected.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // The choice is one member of the triggering entry batch.
    assert!(debug.contains("__zone_change_group__"), "{debug}");
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.matches("SuspectEffect").count() >= 2, "{debug}");
    // Only an actual suspicion clears the source's designation.
    assert!(debug.contains("ClearSuspectedEffect"), "{debug}");
    assert!(debug.contains("SourceSuspected"), "{debug}");
}

#[test]
fn forgers_foundry_mana_offers_exile_instead_of_graveyard_for_small_spells() {
    let definition = compile_to_runtime_definition(
        "Forger's Foundry",
        "Mana cost: {2}{U}\nType: Artifact\n{T}: Add {U}. When you spend this mana to cast an instant or sorcery spell with mana value 3 or less, you may exile that spell instead of putting it into its owner's graveyard as it resolves.\n{3}{U}{U}, {T}: You may cast any number of spells from among cards exiled with this artifact without paying their mana costs. Activate only as a sorcery.",
        false,
    )
    .unwrap();
    let debug = format!("{definition:?}");
    // The spend payload is gated on an instant or sorcery spell with mana
    // value 3 or less.
    assert!(debug.contains("on_spend: [ManaSpendPayload"), "{debug}");
    assert!(debug.contains("LessThanOrEqual(3)"), "{debug}");
    assert!(debug.contains("__mana_paid_object__"), "{debug}");
    // It optionally replaces the spell's resolution move to the graveyard
    // with exile linked to this artifact.
    assert!(debug.contains("link_exiled_to_source: true"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}

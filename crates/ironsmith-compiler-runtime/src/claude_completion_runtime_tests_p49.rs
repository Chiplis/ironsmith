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
    assert!(debug.contains("EventValue(Amount)") || debug.contains("EventValue(\n"), "{debug}");
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

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn tagged_object_reference_cluster_renders_exactly() {
    let cases = [
        (
            "Aurelia's Fury",
            "Aurelia's Fury deals X damage divided as you choose among any number of targets. Tap each creature dealt damage this way. Players dealt damage this way can't cast noncreature spells this turn.",
        ),
        (
            "Dimensional Breach",
            "Exile all permanents. For as long as any of those cards remain exiled, at the beginning of each player's upkeep, that player returns one of the exiled cards they own to the battlefield.",
        ),
        (
            "Guided Passage",
            "Reveal the cards in your library. An opponent chooses from among them a creature card, a land card, and a noncreature, nonland card. You put the chosen cards into your hand. Then shuffle.",
        ),
        (
            "Turtles Forever",
            "Search your library and/or outside the game for exactly four legendary creature cards you own with different names, then reveal those cards. An opponent chooses two of them. Put the chosen cards into your hand and shuffle the rest into your library.",
        ),
        (
            "Threats Undetected",
            "Search your library for up to four creature cards with different powers and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest into your hand.",
        ),
        (
            "Warchanter Skald",
            "Whenever this creature becomes tapped, if it's enchanted or equipped, create a 2/1 red Dwarf Berserker creature token.",
        ),
        (
            "Chaos Defiler",
            "Trample\nBattle Cannon — When this creature enters or dies, for each opponent, choose a nonland permanent that player controls. Destroy one of them chosen at random.",
        ),
        (
            "Ghouls' Night Out",
            "For each player, choose a creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're black Zombies in addition to their other colors and types and they gain decayed.",
        ),
        (
            "Gunner Conscript",
            "Trample\nThis creature gets +1/+1 for each Aura and Equipment attached to it.\nWhen this creature dies, if it was enchanted, create a Junk token.\nWhen this creature dies, if it was equipped, create a Junk token.",
        ),
        (
            "Koll, the Forgemaster",
            "Whenever another nontoken creature you control dies, if it was enchanted or equipped, return it to its owner's hand.\nCreature tokens you control that are enchanted or equipped get +1/+1.",
        ),
        (
            "Model of Unity",
            "Whenever players finish voting, you and each opponent who voted for a choice you voted for may scry 2.\n{T}: Add one mana of any color.",
        ),
        (
            "Stangg",
            "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior creature token. Exile that token when Stangg leaves the battlefield. Sacrifice Stangg when that token leaves the battlefield.",
        ),
        (
            "Cass, Hand of Vengeance",
            "Vigilance\nWhenever Cass or another creature you control dies, if it was enchanted or equipped, return any number of Aura cards that were attached to it from your graveyard to the battlefield attached to target creature, then attach any number of Equipment that were attached to it to that creature.",
        ),
        (
            "Chiss-Goria, Forge Tyrant",
            "Affinity for artifacts\nFlying, haste\nWhenever Chiss-Goria attacks, exile the top five cards of your library. You may cast an artifact spell from among them this turn. If you do, it has affinity for artifacts.",
        ),
        (
            "Cryptic Pursuit",
            "Whenever you cast an instant or sorcery spell from your hand, manifest the top card of your library.\nWhenever a face-down creature you control dies, exile it if it's an instant or sorcery card. You may cast that card until the end of your next turn.",
        ),
        (
            "Foreboding Steamboat",
            "When this Vehicle enters, each player chooses two nontoken, non-Vehicle creatures they control. Exile them until this Vehicle leaves the battlefield.\nWhenever this Vehicle attacks, put a card exiled with it into its owner's graveyard. If you do, investigate.\nCrew 2",
        ),
        (
            "Waltz of Rage",
            "Target creature you control deals damage equal to its power to each other creature. Until end of turn, whenever a creature you control dies, exile the top card of your library. You may play it until the end of your next turn.",
        ),
        (
            "Victimize",
            "Choose two target creature cards in your graveyard. Sacrifice a creature. If you do, return the chosen cards to the battlefield tapped.",
        ),
        (
            "Turf War",
            "When this enchantment enters, for each player, put a contested counter on target land that player controls.\nWhenever a creature deals combat damage to a player, if that player controls one or more lands with contested counters on them, that creature's controller gains control of one of those lands of their choice and untaps it.",
        ),
        (
            "Chandra, Dressed to Kill",
            "+1: Add {R}. Chandra deals 1 damage to up to one target player or planeswalker.\n+1: Exile the top card of your library. If it's red, you may cast it this turn.\n−7: Exile the top five cards of your library. You may cast red spells from among them this turn. You get an emblem with \"Whenever you cast a red spell, this emblem deals X damage to any target, where X is the amount of mana spent to cast that spell.\"",
        ),
        (
            "Soulfire Eruption",
            "Choose any number of target creatures, planeswalkers, and/or players. For each of them, exile the top card of your library, then Soulfire Eruption deals damage equal to that card's mana value to that permanent or player. You may play the exiled cards until the end of your next turn.",
        ),
        (
            "Fire Giant's Fury",
            "Target Giant you control gets +2/+2 and gains trample until end of turn. Whenever it deals combat damage to a player this turn, exile that many cards from the top of your library. Until the end of your next turn, you may play those cards.",
        ),
    ];

    let failures = cases
        .into_iter()
        .filter_map(|(name, expected)| {
            let definition = parse_oracle_card_definition(name);
            let compiled = canonical_compiled_lines(&definition).join("\n");
            (compiled != expected)
                .then(|| format!("{name}:\n  compiled: {compiled:?}\n  expected: {expected:?}"))
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

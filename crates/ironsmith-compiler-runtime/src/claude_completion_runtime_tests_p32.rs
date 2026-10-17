//! Claude package 32 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p32_debug(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

#[test]
fn lure_fish_grants_haste_and_human_block_restriction() {
    let debug = p32_debug(
        "Frost Fair Lure Fish",
        "Mana cost: {5}{U}{R}\nType: Creature — Fish\nPower/Toughness: 7/7\nWhen this creature enters, create two 1/1 blue Fish creature tokens and create two tapped Treasure tokens.\nFish you control have haste and can't be blocked by Humans.\nForetell {3}{U}{R}",
    );
    assert!(debug.contains("Haste"), "Fish keep the haste grant: {debug}");
    assert!(debug.contains("Human"), "the blocker restriction names Humans: {debug}");
    assert!(debug.contains("Fish"), "{debug}");
}

#[test]
fn full_bore_checks_target_warp_receipt() {
    let debug = p32_debug(
        "Full Bore",
        "Mana cost: {R}\nType: Instant\nTarget creature you control gets +3/+2 until end of turn. If that creature was cast for its warp cost, it also gains trample and haste until end of turn.",
    );
    assert!(debug.contains("TargetPaidLabel"), "{debug}");
    assert!(debug.contains("Warp"), "{debug}");
    assert!(debug.contains("Trample") && debug.contains("Haste"), "{debug}");
}

#[test]
fn giant_turtle_attack_restriction_gated_by_last_turn_attack() {
    let debug = p32_debug(
        "Giant Turtle",
        "Mana cost: {1}{G}{G}\nType: Creature — Turtle\nPower/Toughness: 2/4\nThis creature can't attack if it attacked during your last turn.",
    );
    assert!(debug.contains("ObjectAttackedDuringControllersLastTurn"), "{debug}");
}

#[test]
fn gauntlet_of_power_chosen_color_mana_trigger() {
    let debug = p32_debug(
        "Gauntlet of Power",
        "Mana cost: {5}\nType: Artifact\nAs this artifact enters, choose a color.\nCreatures of the chosen color get +1/+1.\nWhenever a basic land is tapped for mana of the chosen color, its controller adds an additional one mana of that color.",
    );
    assert!(
        debug.contains("TriggeringManaIncludesSourceChosenColor"),
        "the trigger keeps its chosen-color qualifier: {debug}"
    );
    assert!(debug.contains("ChosenColor"), "{debug}");
}

#[test]
fn frantic_firebolt_counts_shared_graveyard_selector_list() {
    let debug = p32_debug(
        "Frantic Firebolt",
        "Mana cost: {2}{R}\nType: Instant\nFrantic Firebolt deals X damage to target creature, where X is 2 plus the number of cards in your graveyard that are instant cards, sorcery cards, and/or have an Adventure.",
    );
    assert!(debug.contains("Adventure"), "{debug}");
    assert!(debug.contains("Instant") && debug.contains("Sorcery"), "{debug}");
    assert!(debug.contains("Fixed(2)"), "{debug}");
}

#[test]
fn galactus_must_attack_most_life_opponent_unless_surfer() {
    let debug = p32_debug(
        "Galactus, Devourer of Worlds",
        "Mana cost: {10}\nType: Legendary Creature — Elder Alien\nPower/Toughness: 12/12\nFlying, trample, indestructible\nWhen Galactus enters, exile target permanent.\nInsatiable Hunger — Galactus attacks an opponent with the most life among your opponents each combat if able unless you control a creature named Silver Surfer, Galactus's Herald.",
    );
    assert!(debug.contains("MustAttackPlayer"), "{debug}");
    assert!(debug.contains("MostLifeTiedAmong"), "{debug}");
    assert!(debug.contains("Silver Surfer"), "the unless-condition names the Herald: {debug}");
}

#[test]
fn game_preserve_all_revealed_creature_condition() {
    let debug = p32_debug(
        "Game Preserve",
        "Mana cost: {2}{G}\nType: Enchantment\nAt the beginning of your upkeep, each player reveals the top card of their library. If all cards revealed this way are creature cards, put those cards onto the battlefield under their owners' control.",
    );
    assert!(debug.contains("excluded_card_types: [Creature]"), "{debug}");
}

#[test]
fn found_footage_face_down_look_permission() {
    let debug = p32_debug(
        "Found Footage",
        "Mana cost: {1}\nType: Artifact — Clue\nYou may look at face-down creatures your opponents control any time.\n{2}, Sacrifice this artifact: Surveil 2, then draw a card. (To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)",
    );
    assert!(debug.contains("LookAtFaceDownCreaturesYouDontControl"), "{debug}");
}

#[test]
fn foul_emissary_emerge_sacrifice_qualifier() {
    let debug = p32_debug(
        "Foul Emissary",
        "Mana cost: {2}{G}\nType: Creature — Human Horror\nPower/Toughness: 1/1\nWhen this creature enters, look at the top four cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.\nWhen you sacrifice this creature while casting a spell with emerge, create a 3/2 colorless Eldrazi Horror creature token.",
    );
    assert!(debug.contains("TriggeringSacrificePaidEmergeCost"), "{debug}");
}

#[test]
fn ghastly_conscription_manifests_shuffled_graveyard_pile() {
    let debug = p32_debug(
        "Ghastly Conscription",
        "Mana cost: {5}{B}{B}\nType: Sorcery\nExile all creature cards from target player's graveyard in a face-down pile, shuffle that pile, then manifest those cards. (To manifest a card, put it onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)",
    );
    assert!(debug.contains("ManifestObjects"), "{debug}");
    assert!(debug.contains("shuffle: true"), "{debug}");
}

#[test]
fn fractured_powerstone_rolls_planar_die() {
    let debug = p32_debug(
        "Fractured Powerstone",
        "Mana cost: {2}\nType: Artifact\n{T}: Add {C}.\n{T}: Roll the planar die. Activate only as a sorcery.",
    );
    assert!(debug.contains("RollPlanarDie"), "{debug}");

    // Outside a Planechase game there is no planar die: the instruction is a
    // no-op rather than an error.
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::{PlayerId, Zone};
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_card(
        &ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Powerstone Probe")
            .card_types(vec![ironsmith::types::CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let effect = Effect::new(ironsmith::effects::RollPlanarDieEffect::new(
        ironsmith::target::PlayerFilter::You,
    ));
    let mut ctx = EffectContext::new_default(source, alice);
    execute_effect(&mut game, &effect, &mut ctx).expect("no planar die outside Planechase");
}

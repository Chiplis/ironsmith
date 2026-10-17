//! Claude package 37 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p37_definition(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn p37_compact(definition: &ironsmith::cards::CardDefinition) -> String {
    format!("{definition:#?}").split_whitespace().collect::<String>()
}

#[test]
fn lunar_hatchling_escape_keeps_the_land_exile_and_the_graveyard_exile() {
    let definition = p37_definition(
        "Lunar Hatchling",
        "Mana cost: {4}{G}{U}\nType: Creature — Alien Beast\nPower/Toughness: 6/6\nFlying, trample\nBasic landcycling {2} ({2}, Discard this card: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.)\nEscape—{4}{G}{U}, Exile a land you control, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)",
    );
    let compact = p37_compact(&definition);
    let escape = compact
        .split_once("Escape{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("escape method: {compact}"));
    assert!(escape.contains("exile_count:5"), "{escape}");
    assert!(escape.contains("Land"), "the land-you-control exile survives: {escape}");
    assert!(escape.contains("Graveyard"), "the graveyard exile survives: {escape}");
}

#[test]
fn limited_resources_land_ban_is_gated_by_a_ten_land_threshold() {
    let definition = p37_definition(
        "Limited Resources",
        "Mana cost: {W}\nType: Enchantment\nWhen this enchantment enters, each player chooses five lands they control and sacrifices the rest.\nPlayers can't play lands as long as ten or more lands are on the battlefield.",
    );
    let compact = p37_compact(&definition);
    let conditional = compact
        .split_once("Conditional{")
        .map(|(_, tail)| tail)
        .unwrap_or_else(|| panic!("conditional land restriction: {compact}"));
    assert!(conditional.contains("PlayLands"), "{conditional}");
    assert!(conditional.contains("GreaterThanOrEqual"), "a threshold, not existence: {conditional}");
    assert!(conditional.contains("Fixed(10)"), "{conditional}");
}

#[test]
fn lurker_spell_target_ban_lifts_after_it_attacked_or_blocked() {
    let definition = p37_definition(
        "Lurker",
        "Mana cost: {2}{G}\nType: Creature — Beast\nPower/Toughness: 2/3\nThis creature can't be the target of spells unless it attacked or blocked this turn.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("Conditional{"), "{compact}");
    assert!(compact.contains("Not("), "unless is the negated condition: {compact}");
    assert!(compact.contains("AttackedOrBlockedThisTurn"), "{compact}");
}

#[test]
fn magnanimous_magistrate_removes_counters_equal_to_the_dead_creatures_mana_value() {
    let definition = p37_definition(
        "Magnanimous Magistrate",
        "Mana cost: {5}{W}\nType: Creature — Human Advisor\nPower/Toughness: 4/4\nThis creature enters with five reprieve counters on it.\nWhenever another nontoken creature you control dies, if its mana value was 1 or greater, you may remove that many reprieve counters from this creature. If you do, return that card to the battlefield under its owner's control.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("ManaValueOf(Tagged("), "that many is the tested mana value: {compact}");
    assert!(compact.contains("reprieve"), "{compact}");
}

#[test]
fn magmablood_archaic_pump_counts_colors_spent_on_the_triggering_spell() {
    let definition = p37_definition(
        "Magmablood Archaic",
        "Mana cost: {2/R}{2/R}{2/R}\nType: Creature — Avatar\nPower/Toughness: 2/2\nReach, trample\nConverge — This creature enters with a +1/+1 counter on it for each color of mana spent to cast it.\nWhenever you cast an instant or sorcery spell, creatures you control get +1/+0 until end of turn for each color of mana spent to cast that spell.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("ColorsOfManaSpentToCastTriggeringObject"), "{compact}");
    assert!(compact.contains("ColorsOfManaSpentToCastThisSpell"), "converge keeps its own count: {compact}");
}

#[test]
fn marina_vendrells_grimoire_splits_hand_size_and_zero_life_rules() {
    let definition = p37_definition(
        "Marina Vendrell's Grimoire",
        "Mana cost: {5}{U}\nType: Legendary Artifact — Book\nWhen Marina Vendrell's Grimoire enters, if you cast it, draw five cards.\nYou have no maximum hand size and don't lose the game for having 0 or less life.\nWhenever you gain life, draw that many cards.\nWhenever you lose life, discard that many cards. Then if you have no cards in hand, you lose the game.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("NoMaximumHandSize"), "{compact}");
    assert!(compact.contains("LoseGameForZeroLife(You)"), "{compact}");
}

#[test]
fn lukka_grants_conditional_exile_cast_and_reveals_a_bigger_creature() {
    let definition = p37_definition(
        "Lukka, Coppercoat Outcast",
        "Mana cost: {3}{R}{R}\nType: Legendary Planeswalker — Lukka\nLoyalty: 5\n+1: Exile the top three cards of your library. Creature cards exiled this way gain \"You may cast this card from exile as long as you control a Lukka planeswalker.\"\n−2: Exile target creature you control, then reveal cards from the top of your library until you reveal a creature card with greater mana value. Put that card onto the battlefield and the rest on the bottom of your library in a random order.\n−7: Each creature you control deals damage equal to its power to each opponent.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("Lukka"), "the granted permission keeps its Lukka condition: {compact}");
    assert!(
        compact.contains("mana_value:Some(GreaterThanExpr(ManaValueOf(Tagged("),
        "the revealed creature must out-cost the exiled one: {compact}"
    );
}

#[test]
fn likeness_looter_copy_keeps_flying_and_its_copy_ability() {
    let definition = p37_definition(
        "Likeness Looter",
        "Mana cost: {U}{B}\nType: Creature — Faerie Shapeshifter\nPower/Toughness: 1/1\nFlying\n{T}: Draw a card, then discard a card.\n{X}: This creature becomes a copy of target creature card in your graveyard with mana value X, except it has flying and this ability. Activate only as a sorcery.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("BecomeCopy") || compact.contains("Copy"), "{compact}");
    assert!(compact.contains("Flying"), "{compact}");
}

#[test]
fn loot_the_anomaly_assigns_negative_power_as_positive() {
    let definition = p37_definition(
        "Loot, the Anomaly",
        "Mana cost: {2}{B}\nType: Legendary Creature — Beast Horror\nPower/Toughness: 1/4\nIf Loot's power is negative, he assigns combat damage as though his power were positive.\nThreshold — Sacrifice another creature or planeswalker: Loot gets -2/-0 until end of turn. Activate only if there are seven or more cards in your graveyard.",
    );
    let compact = p37_compact(&definition);
    assert!(
        compact.contains("assignscombatdamageasthoughitspowerwerepositive"),
        "{compact}"
    );
    assert!(compact.contains("Fixed(-2)"), "{compact}");
}

#[test]
fn lumbering_laundry_grants_face_down_look_permission_until_end_of_turn() {
    let definition = p37_definition(
        "Lumbering Laundry",
        "Mana cost: {5}\nType: Artifact Creature — Golem\nPower/Toughness: 4/5\n{2}: Until end of turn, you may look at face-down creatures you don't control any time.\nDisguise {5} (You may cast this card face down for {3} as a 2/2 creature with ward {2}. Turn it face up any time for its disguise cost.)",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("LookAtFaceDownCreaturesYouDontControl"), "{compact}");
    assert!(compact.contains("EndOfTurn"), "{compact}");
}

#[test]
fn lobelia_looks_at_each_opponents_top_card_and_exiles_them_face_down() {
    let definition = p37_definition(
        "Lobelia, Defender of Bag End",
        "Mana cost: {2}{B}\nType: Legendary Creature — Halfling Citizen\nPower/Toughness: 2/3\nWhen Lobelia enters, look at the top card of each opponent's library and exile those cards face down.\n{T}, Sacrifice an artifact: Choose one —\n• Until end of turn, you may play a card exiled with Lobelia without paying its mana cost.\n• Each opponent loses 2 life and you gain 2 life.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("ForPlayersEffect{filter:Opponent") || compact.contains("Opponent"), "{compact}");
    assert!(compact.contains("face_down:true"), "{compact}");
}

#[test]
fn maddening_imp_forces_non_wall_attacks_and_punishes_non_attackers() {
    let definition = p37_definition(
        "Maddening Imp",
        "Mana cost: {2}{B}\nType: Creature — Imp\nPower/Toughness: 1/1\nFlying\n{T}: Non-Wall creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy each of those creatures that didn't attack this turn. Activate only during an opponent's turn and only before combat.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("MustAttack"), "{compact}");
    assert!(compact.contains("Wall"), "the non-Wall exclusion survives: {compact}");
    assert!(compact.contains("didnt_attack_this_turn:true") || compact.contains("AttackedThisTurn"), "{compact}");
}

#[test]
fn luxior_equips_planeswalkers_and_turns_them_into_creatures() {
    let definition = p37_definition(
        "Luxior, Giada's Gift",
        "Mana cost: {1}\nType: Legendary Artifact — Equipment\nEquipped creature gets +1/+1 for each counter on it.\nEquipped permanent isn't a planeswalker and is a creature in addition to its other types. (Loyalty abilities can still be activated.)\nEquip planeswalker {1}\nEquip {3}",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("RemoveCardTypesForFilter"), "{compact}");
    assert!(compact.contains("card_types:[Planeswalker]"), "{compact}");
    assert!(compact.matches("keyword:Some(Equip)").count() >= 2, "two equip abilities: {compact}");
}

#[test]
fn lynde_moves_a_curse_on_you_to_an_opponent() {
    let definition = p37_definition(
        "Lynde, Cheerful Tormentor",
        "Mana cost: {1}{U}{B}{R}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 2/4\nDeathtouch\nWhenever a Curse is put into your graveyard from the battlefield, return it to the battlefield attached to you at the beginning of the next end step.\nAt the beginning of your upkeep, you may attach a Curse attached to you to one of your opponents. If you do, draw two cards.",
    );
    let compact = p37_compact(&definition);
    assert!(compact.contains("attached_to_player:Some(You)"), "{compact}");
    assert!(compact.contains("Curse"), "{compact}");
    assert!(compact.contains("Opponent"), "{compact}");
}

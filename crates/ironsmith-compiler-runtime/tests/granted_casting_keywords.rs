//! Casting keywords granted to cards or spells through one shared grant path:
//! warp (CR 702.185), prowl (CR 702.76), freerunning (CR 702.173), miracle
//! (CR 702.94) from hand, and jump-start (CR 702.133) from the graveyard.
//! Source-authored, deliberately unrun.
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p02_line_families/compile.rs"]
mod compile;

const CARDS: &[(&str, &str, &str, &str)] = &[
    (
        "Tannuk, Steadfast Second",
        "Mana cost: {2}{R}{R}\nType: Legendary Creature — Kavu Pilot\nPower/Toughness: 3/5\nOther creatures you control have haste.\nArtifact cards and red creature cards in your hand have warp {2}{R}. (You may cast a card from your hand for its warp cost. Exile that permanent at the beginning of the next end step, then you may cast it from exile on a later turn.)",
        "Warp",
        "zone: Hand",
    ),
    (
        "Hunting Velociraptor",
        "Mana cost: {2}{R}\nType: Creature — Dinosaur\nPower/Toughness: 3/2\nFirst strike\nDinosaur spells you cast have prowl {2}{R}. (You may cast a spell for its prowl cost if you dealt combat damage to a player this turn with a creature with any of its creature types.)",
        "Prowl",
        "zone: Hand",
    ),
    (
        "Ezio Auditore da Firenze",
        "Mana cost: {1}{B}\nType: Legendary Creature — Human Assassin\nPower/Toughness: 3/2\nMenace\nAssassin spells you cast have freerunning {B}{B}. (You may cast a spell for its freerunning cost if you dealt combat damage to a player this turn with an Assassin or commander.)\nWhenever Ezio deals combat damage to a player, you may pay {W}{U}{B}{R}{G} if that player has 10 or less life. When you do, that player loses the game.",
        "Freerunning",
        "zone: Hand",
    ),
    (
        "Lorehold, the Historian",
        "Mana cost: {3}{R}{W}\nType: Legendary Creature — Elder Dragon\nPower/Toughness: 5/5\nFlying, haste\nEach instant and sorcery card in your hand has miracle {2}. (You may cast a card for its miracle cost when you draw it if it's the first card you drew this turn.)\nAt the beginning of each opponent's upkeep, you may discard a card. If you do, draw a card.",
        "Miracle",
        "zone: Hand",
    ),
    (
        "Niv-Mizzet, Supreme",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Creature — Dragon Avatar\nPower/Toughness: 5/5\nFlying, hexproof from monocolored\nEach instant and sorcery card in your graveyard that's exactly two colors has jump-start. (You may cast that card from your graveyard by discarding a card in addition to paying its other costs. Then exile it.)",
        "JumpStart",
        "zone: Graveyard",
    ),
];

#[test]
fn granted_casting_keywords_compile_to_zone_scoped_alternative_cast_grants() {
    for &(name, text, method, zone) in CARDS {
        for definition in compile::compile_both(name, text) {
            let grants = compile::statics(&definition, StaticAbilityId::Grants);
            let grant = grants
                .iter()
                .map(|ability| format!("{ability:?}"))
                .find(|debug| debug.contains(method))
                .unwrap_or_else(|| panic!("{name}: granted {method}"));
            assert!(grant.contains(zone), "{name}: {grant}");
            assert!(!grant.contains("PlayFrom"), "{name}: no zone permission: {grant}");
        }
    }
}

#[test]
fn granted_prowl_and_freerunning_keep_their_casting_conditions() {
    for &(name, text, method, _) in &CARDS[1..3] {
        for definition in compile::compile_both(name, text) {
            let grants = compile::statics(&definition, StaticAbilityId::Grants);
            let grant = grants
                .iter()
                .map(|ability| format!("{ability:?}"))
                .find(|debug| debug.contains(method))
                .unwrap();
            assert!(grant.contains("YouDealtCombatDamageToPlayer"), "{name}: {grant}");
        }
    }
}

//! Full-card direct/artifact regressions for exact-reader ownership and cost boundaries.
#[path = "p01_support/mod.rs"]
mod support;
macro_rules! card_reader {
    ($test:ident, $name:literal, $text:literal) => {
        #[test]
        fn $test() { support::definitions_for_text($name, $text); }
    };
}
card_reader!(compleat_devotion,"Compleat Devotion","Mana cost: {1}{W}\nType: Instant\nTarget creature you control gets +2/+2 until end of turn. If that creature has toxic, draw a card.");
card_reader!(hexgold_slash,"Hexgold Slash","Mana cost: {R}\nType: Instant\nHexgold Slash deals 2 damage to target creature. If that creature has toxic, Hexgold Slash deals 4 damage to that creature instead.");
card_reader!(arachnus_web,"Arachnus Web","Mana cost: {2}{G}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature can't attack or block, and its activated abilities can't be activated.\nAt the beginning of the end step, if enchanted creature's power is 4 or greater, destroy this Aura.");
card_reader!(domestication,"Domestication","Mana cost: {2}{U}{U}\nType: Enchantment — Aura\nEnchant creature\nYou control enchanted creature.\nAt the beginning of your end step, if enchanted creature's power is 4 or greater, sacrifice this Aura.");
card_reader!(mysterious_pathlighter,"Mysterious Pathlighter","Mana cost: {2}{W}\nType: Creature — Faerie\nPower/Toughness: 2/2\nFlying\nEach creature you control that has an Adventure enters with an additional +1/+1 counter on it. (It doesn't need to have gone on the adventure first.)");
card_reader!(edgar_master_machinist,"Edgar, Master Machinist","Mana cost: {2}{R}{W}\nType: Legendary Creature — Human Artificer Noble\nPower/Toughness: 2/4\nOnce during each of your turns, you may cast an artifact spell from your graveyard. If you cast a spell this way, that artifact enters tapped.\nTools — Whenever Edgar attacks, it gets +X/+0 until end of turn, where X is the greatest mana value among artifacts you control.");
card_reader!(detective_s_phoenix,"Detective's Phoenix","Mana cost: {2}{R}\nType: Enchantment Creature — Phoenix\nPower/Toughness: 2/2\nBestow—{R}, Collect evidence 6. (To pay this bestow cost, pay {R} and exile cards with total mana value 6 or greater from your graveyard.)\nFlying, haste\nEnchanted creature gets +2/+2 and has flying and haste.\nYou may cast this card from your graveyard using its bestow ability.");
card_reader!(assassin_s_ink,"Assassin's Ink","Mana cost: {2}{B}{B}\nType: Instant\nThis spell costs {1} less to cast if you control an artifact and {1} less to cast if you control an enchantment.\nDestroy target creature or planeswalker.");
card_reader!(geistlight_snare,"Geistlight Snare","Mana cost: {2}{U}\nType: Instant\nThis spell costs {1} less to cast if you control a Spirit. It also costs {1} less to cast if you control an enchantment.\nCounter target spell unless its controller pays {3}.");

#[test]
fn aluren_preserves_its_permission() {
    for definition in support::definitions_for_text("Aluren","Mana cost: {2}{G}{G}\nType: Enchantment\nAny player may cast creature spells with mana value 3 or less without paying their mana costs and as though they had flash.") {
        let text = support::rendered(&definition);
        assert!(text.contains("without paying"), "{text}");
        assert!(definition.spell_effect.is_none(), "permanent permission must be static");
        assert_eq!(definition.abilities.len(), 2);
    }
}

#[test]
fn atomic_microsizer_preserves_its_permission() {
    for definition in support::definitions_for_text("Atomic Microsizer","Mana cost: {U}\nType: Artifact — Equipment\nEquipped creature gets +1/+0.\nWhenever equipped creature attacks, choose up to one target creature. That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn.\nEquip {2}") {
        let text = support::rendered(&definition);
        assert!(text.contains("can't be blocked"), "{text}");
        assert!(text.contains("base power and toughness 1/1"), "{text}");
    }
}

#[test]
fn aluren_allows_opponents_to_cast_only_small_creatures_for_free_outside_main_phase() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::{GameState, PlayerId, Zone};
    let alice = PlayerId(0);
    let bob = PlayerId(1);
    for definition in support::definitions_for_text("Aluren", "Mana cost: {2}{G}{G}\nType: Enchantment\nAny player may cast creature spells with mana value 3 or less without paying their mana costs and as though they had flash.") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = alice;
        game.turn.priority_player = Some(bob);
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::Step::Upkeep);
        game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for (name, text, expected) in [
            ("Small Bear", "Mana cost: {2}{G}\nType: Creature — Bear\nPower/Toughness: 2/2", true),
            ("Large Bear", "Mana cost: {3}{G}\nType: Creature — Bear\nPower/Toughness: 2/2", false),
            ("Sorcery", "Mana cost: {G}\nType: Sorcery\nDraw a card.", false),
        ] {
            let card = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
            let id = game.create_object_from_definition(&card, bob, Zone::Hand);
            let has_free_cast = compute_legal_actions(&game, bob).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == id));
            assert_eq!(has_free_cast, expected, "{name}: empty mana pool, opponent's upkeep");
        }
    }
}

card_reader!(ageless_sentinels,"Ageless Sentinels","Mana cost: {3}{W}\nType: Creature — Wall\nPower/Toughness: 4/4\nDefender (This creature can't attack.)\nFlying\nWhen this creature blocks, it becomes a Bird Giant, and it loses defender. (It's no longer a Wall. This effect lasts indefinitely.)");

card_reader!(emrakul_the_promised_end,"Emrakul, the Promised End","Mana cost: {13}\nType: Legendary Creature — Eldrazi\nPower/Toughness: 13/13\nThis spell costs {1} less to cast for each card type among cards in your graveyard.\nWhen you cast this spell, you gain control of target opponent during that player's next turn. After that turn, that player takes an extra turn.\nFlying, trample, protection from instants");

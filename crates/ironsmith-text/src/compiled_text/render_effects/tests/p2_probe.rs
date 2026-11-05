use super::*;

fn dump(name: &str, type_line: &str, text: &str) {
    let def = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .parse_text(format!("Type: {type_line}\n{text}"));
    match def {
        Ok(def) => {
            eprintln!("P2PROBE {name} LINES {:?}", crate::compiled_text::compiled_text_lines(&def));
            for (i, a) in def.abilities.iter().enumerate() {
                let s = format!("{:?}", a.kind);
                eprintln!("P2PROBE {name} ABILITY {i}: {}", &s[..s.len().min(6000)]);
            }
            if let Some(spell) = &def.spell_effect {
                let s = format!("{spell:?}");
                eprintln!("P2PROBE {name} SPELL: {}", &s[..s.len().min(6000)]);
            }
        }
        Err(e) => eprintln!("P2PROBE {name} ERR {e:?}"),
    }
}

#[test]
fn p2_probe_text() {
    dump("Urza's Workshop", "Land — Urza's", "Metalcraft — {T}: Add {C} for each Urza's land you control. Activate only if you control three or more artifacts.");
    dump("Combat Ladder", "Creature", "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying, deathtouch, double strike, haste, hexproof, indestructible, lifelink, menace, reach, skulk, trample, and vigilance.");
    dump("Bleeding Effect", "Enchantment", "At the beginning of combat on your turn, creatures you control gain flying until end of turn if a creature card in your graveyard has flying. The same is true for first strike, double strike, deathtouch, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance.");
    dump("Cast result probe", "Sorcery", "Target opponent mills nine cards, then you may cast an instant or sorcery spell from that player's graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead.");
    dump("Graveyard cast", "Creature", "Whenever this creature attacks, mill four cards. You may cast an instant or sorcery spell from your graveyard with mana value 4 or less without paying its mana cost. If that spell would be put into your graveyard, exile it instead.");
    dump("Bre of Clan Stoutarm", "Creature", "At the beginning of your end step, if you gained life this turn, exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is less than or equal to the amount of life you gained this turn. Otherwise, put it into your hand.");
    dump("Envoy Probe", "Instant", "Look at the top card of your library. You may play a land from the top of your library or cast a spell with mana value 3 or less from the top of your library without paying its mana cost. If you don't, put that card into your hand.");
    dump("Source Damage Probe", "Enchantment", "At the beginning of your end step, target enchantment deals damage equal to its mana value to its controller unless that player sacrifices it.");
    dump("Consult cast", "Sorcery", "Exile cards from the top of your library until you exile cards with total mana value 4 or greater. You may cast any number of spells from among them without paying their mana costs.");
    dump("Pile attacker", "Creature", "Whenever this creature attacks, look at the top three cards of your library. Exile any number of them in a face-down pile and the rest in a face-up pile. Defending player chooses one of those piles. Put that pile into your hand.\nAt the beginning of each end step, each opponent loses 2 life for each spell or ability that caused one or more players to guess or to group cards or permanents into a pile this turn.");
    dump("Extremum Payment Probe", "Sorcery", "Each player sacrifices the creature they control with the greatest mana value unless they pay that creature's mana cost. If two or more creatures a player controls are tied for greatest, that player chooses one.");
    dump("Bound by Moonsilver", "Enchantment — Aura", "Enchant creature\nEnchanted creature can't attack, block, or transform.");
    dump("Relic Runner", "Creature", "This creature can't be blocked if you've cast a historic spell this turn.");
    dump("Clinging Mists", "Instant", "Prevent all combat damage that would be dealt this turn.\nFateful hour — If you have 5 or less life, tap all attacking creatures. Those creatures don't untap during their controller's next untap step.");
}

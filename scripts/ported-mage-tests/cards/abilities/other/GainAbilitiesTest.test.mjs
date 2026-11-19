import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/GainAbilitiesTest.java",
  "tests": [
    {
      "name": "test_AttachmentSingleton",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abzan Runemark@first",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abzan Runemark@second",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears@bear",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Abzan Runemark",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@first",
          "target": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Abzan Runemark",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "@first",
          "target": "@bear",
          "expected": true
        },
        {
          "op": "castSpell",
          "name": "Abzan Runemark",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@second",
          "target": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Abzan Runemark",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "@second",
          "target": "@bear",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Balduvian Bears",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@bear"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "Vigilance",
              "expected": true
            }
          ]
        },
        {
          "op": "attack",
          "turn": 1,
          "attacker": "@bear",
          "player": 0,
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "@bear",
          "tapped": false
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 14
        }
      ]
    },
    {
      "name": "test_AttachmentUnique",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Epiphany Storm@first",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Epiphany Storm@second",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Balduvian Bears@bear",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Epiphany Storm",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@first",
          "target": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Epiphany Storm",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "@first",
          "target": "@bear",
          "expected": true
        },
        {
          "op": "castSpell",
          "name": "Epiphany Storm",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@second",
          "target": "@bear"
        },
        {
          "op": "assertStackObject",
          "source": "Epiphany Storm",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "@second",
          "target": "@bear",
          "expected": true
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "@bear",
          "kind": "activated",
          "textPattern": "Draw a card",
          "count": 2
        }
      ]
    },
    {
      "name": "testGainAbilityControlledSpells",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Hoarding Broodlord",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Firesong and Sunspeaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Covetous Urge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Pestilent Spirit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Hoarding Broodlord"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Shock"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Covetous Urge",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Pestilent Spirit"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}{C}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pestilent Spirit"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hoarding Broodlord"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": "Firesong and Sunspeaker"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Firesong and Sunspeaker"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Firesong and Sunspeaker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Hoarding Broodlord\", true)"
        }
      ]
    },
    {
      "name": "testGainAbilityControlledSpellsOnly",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Firesong and Sunspeaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Covetous Urge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Searing Blood",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Covetous Urge",
          "target": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Searing Blood"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Searing Blood",
          "target": "Walking Corpse",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Searing Blood",
          "count": 1,
          "isSpell": true,
          "controller": 0,
          "objectTarget": "Walking Corpse"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "nextSpellCastHasConvoke",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wand of the Worldsoul",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: The next spell you cast this turn"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elvish Mystic"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Runeclaw Bear"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Runeclaw Bear\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        }
      ]
    },
    {
      "name": "onlyNextSpellCastHasConvoke",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wand of the Worldsoul",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Piker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: The next spell you cast this turn"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: The next spell you cast this turn"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Goblin Piker"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Elvish Mystic",
          "expected": false
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Goblin Piker\", true)"
        }
      ]
    },
    {
      "name": "gainAbilitiesDontRemoveEachOther",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Frondland Felidar",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Jubilant Skybonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Proud Wildbonder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alloy Myr",
          "count": 11
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Frondland Felidar"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Jubilant Skybonder"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Proud Wildbonder"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Frondland Felidar",
          "count": 1
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Frondland Felidar",
          "kind": "static",
          "textPattern": "creature(?:s)? you control with vigilance",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Jubilant Skybonder",
          "count": 1
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Jubilant Skybonder",
          "kind": "static",
          "textPattern": "creature(?:s)? you control with flying",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Proud Wildbonder",
          "count": 1
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Proud Wildbonder",
          "kind": "static",
          "textPattern": "creature(?:s)? you control with trample",
          "count": 1
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Frondland Felidar",
          "kind": "activated",
          "textPattern": "tap target creature",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Jubilant Skybonder",
          "ability": "Flying",
          "count": 1
        },
        {
          "op": "assertKeywordCount",
          "player": 0,
          "name": "Proud Wildbonder",
          "ability": "Trample",
          "count": 1
        }
      ]
    }
  ]
});

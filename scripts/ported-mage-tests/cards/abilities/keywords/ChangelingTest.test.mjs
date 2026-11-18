import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ChangelingTest.java",
  "tests": [
    {
      "name": "testLongForgottenGohei",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Woodland Changeling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Long-Forgotten Gohei",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Woodlan",
          "expected": false
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Woodland Changeling",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Woodland Changeling",
          "count": 1
        }
      ]
    },
    {
      "name": "testGainingChangeling",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prophet of Kruphix",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Amoeboid Changeling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hibernation Sliver",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prophet of Kruphix"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target creature gains",
          "target": "Prophet of Kruphix"
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
          "op": "unsupported",
          "source": "assertTapped(\"Amoeboid Changeling\", true)"
        }
      ]
    },
    {
      "name": "kasetoOrochiArchmageSnakeTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kaseto, Orochi Archmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chameleon Colossus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nessian Asp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}{U}: Target creature can't be blocked this turn. If that creature is a Snake, it gets +2/+2 until end of turn",
          "target": "Chameleon Colossus"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}{U}: Target creature can't be blocked this turn. If that creature is a Snake, it gets +2/+2 until end of turn",
          "target": "Nessian Asp"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Nessian Asp",
          "power": 6,
          "toughness": 7
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Chameleon Colossus",
          "power": 6,
          "toughness": 6
        }
      ]
    },
    {
      "name": "testLoseAllCreatureTypes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Game-Trail Changeling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Chieftain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nameless Inversion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Game-Trail Changeling"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "changeling",
              "expected": true
            }
          ]
        },
        {
          "op": "castSpell",
          "name": "Nameless Inversion",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Game-Trail Changeling"
        },
        {
          "op": "assertStackObject",
          "source": "Nameless Inversion",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectTarget": "Game-Trail Changeling"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Game-Trail Changeling"
          },
          "count": 1,
          "checks": [
            {
              "property": "power",
              "expected": 7
            },
            {
              "property": "toughness",
              "expected": 1
            },
            {
              "property": "subtype",
              "expected": false,
              "value": "Shapeshifter"
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Game-Trail Changeling",
          "ability": "Haste",
          "expected": false
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Game-Trail Changeling"
          },
          "count": 1,
          "checks": [
            {
              "property": "keyword",
              "value": "changeling",
              "expected": true
            }
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Game-Trail Changeling"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "expected": true,
              "value": "Shapeshifter"
            },
            {
              "property": "subtype",
              "expected": true,
              "value": "Goblin"
            }
          ]
        }
      ]
    },
    {
      "name": "testLoseAbilities",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Game-Trail Changeling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Merfolk Trickster"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Game-Trail Changeling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "unsupported",
          "source": "assertTapped(\"Game-Trail Changeling\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Game-Trail Changeling\", SubType.GOBLIN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Game-Trail Changeling\", SubType.ELF)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Game-Trail Changeling\", SubType.SHAPESHIFTER)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Game-Trail Changeling",
          "ability": "new ChangelingAbility()",
          "expected": false
        }
      ]
    }
  ]
});

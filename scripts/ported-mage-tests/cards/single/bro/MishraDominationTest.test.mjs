import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bro/MishraDominationTest.java",
  "tests": [
    {
      "name": "test_Boost",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mishra's Domination",
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
          "name": "Mishra's Domination",
          "target": "Memnite"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Grizzly Bears",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Memnite",
          "attacker": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Threaten_Boosted",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mishra's Domination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Threaten",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mishra's Domination",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 1,
          "toughness": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Threaten",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_CantBlock",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mishra's Domination",
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
          "name": "Mishra's Domination",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 1,
          "toughness": "Grizzly Bears"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Grizzly Bears",
          "attacker": "Memnite"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});

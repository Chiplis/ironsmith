import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/prevention/TestOfFaithTest.java",
  "tests": [
    {
      "name": "testOneAttackerOneBlockerUsingFaith",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Faith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blur Sliver",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Faith",
          "target": "Soulmender"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Blur Sliver",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Blur Sliver"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soulmender",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Blur Sliver",
          "count": 1
        }
      ]
    },
    {
      "name": "testOneAttackerTwoBlockerOneUsingFaith",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Faith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kalonian Tusker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Faith",
          "target": "Soulmender"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kalonian Tusker",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Elvish Mystic",
          "attacker": "Kalonian Tusker"
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Kalonian Tusker"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soulmender",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kalonian Tusker",
          "count": 1
        }
      ]
    },
    {
      "name": "testOneAttackerTwoBlockerTwoUsingFaith",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Test of Faith",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kalonian Tusker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Faith",
          "target": "Soulmender"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Test of Faith",
          "target": "Elvish Mystic"
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Kalonian Tusker",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Elvish Mystic",
          "attacker": "Kalonian Tusker"
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Soulmender",
          "attacker": "Kalonian Tusker"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soulmender",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Soulmender",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elvish Mystic",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kalonian Tusker",
          "count": 1
        }
      ]
    }
  ]
});

import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/DashTest.java",
  "tests": [
    {
      "name": "testDash",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {1}{R} (source: Screamreach Brawler"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Screamreach Brawler",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        }
      ]
    },
    {
      "name": "testNoDash",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_NORMAL_COST"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "attack: Scream",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        }
      ]
    },
    {
      "name": "testDashedCreatureDiesInCombat",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Geist of the Moors",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {1}{R} (source: Screamreach Brawler"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Screamreach Brawler",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Geist of the Moors",
          "attacker": "Screamreach Brawler"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Geist of the Moors",
          "count": 1
        }
      ]
    },
    {
      "name": "testDashedCreatureDiesInCombatAndIsLaterRecast",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "zone": "HAND",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {1}{R} (source: Screamreach Brawler"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Screamreach Brawler",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {1}{R} (source: Screamreach Brawler"
        },
        {
          "op": "setStopAt",
          "turn": 3,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Screamreach Brawler",
          "ability": "Haste",
          "expected": true
        }
      ]
    },
    {
      "name": "testWarbringerCostReduction",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Warbringer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Warbringer",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Warbringer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {2}{R} (source: Warbringer"
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
          "name": "Warbringer",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Warbringer",
          "count": 0
        }
      ]
    },
    {
      "name": "testRegularCostReduction",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ruby Medallion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Screamreach Brawler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Dash alternative cost: {1}{R} (source: Screamreach Brawler"
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
          "name": "Ruby Medallion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Screamreach Brawler",
          "count": 0
        }
      ]
    }
  ]
});

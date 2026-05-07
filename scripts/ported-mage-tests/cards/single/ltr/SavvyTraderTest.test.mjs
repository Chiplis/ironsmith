import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/SavvyTraderTest.java",
  "tests": [
    {
      "name": "test_Play_Baneslayer",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Savvy Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Savvy Trader"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Baneslayer Angel"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Baneslayer Angel"
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
          "op": "assertTappedCount",
          "name": "Savannah",
          "tapped": true,
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Baneslayer Angel",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Play_Swamp",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Savvy Trader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Savvy Trader"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Swamp"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swamp"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        }
      ]
    }
  ]
});

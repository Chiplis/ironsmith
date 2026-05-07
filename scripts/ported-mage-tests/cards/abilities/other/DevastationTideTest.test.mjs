import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/DevastationTideTest.java",
  "tests": [
    {
      "name": "testReturnNonLandPermanents",
      "operations": [
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
          "player": 0,
          "name": "Devastation Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampiric Rites",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hedron Archive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Karn Liberated",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nimbus Maze",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vampiric Rites",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hedron Archive",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Karn Liberated",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Nimbus Maze",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devastation Tide"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Vampiric Rites",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Hedron Archive",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Karn Liberated",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Nimbus Maze",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Vampiric Rites",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Hedron Archive",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Karn Liberated",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Nimbus Maze",
          "count": 0
        }
      ]
    }
  ]
});

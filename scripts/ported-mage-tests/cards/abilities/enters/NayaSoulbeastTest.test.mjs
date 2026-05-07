import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/NayaSoulbeastTest.java",
  "tests": [
    {
      "name": "testNayaEntersWithTwoCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Naya Soulbeast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Naya Soulbeast"
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
          "name": "Naya Soulbeast",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Naya Soulbeast",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Naya Soulbeast",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});

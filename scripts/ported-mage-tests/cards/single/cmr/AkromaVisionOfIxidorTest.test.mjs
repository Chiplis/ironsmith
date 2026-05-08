import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/AkromaVisionOfIxidorTest.java",
  "tests": [
    {
      "name": "test_MustBoostCreatures",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma, Vision of Ixidor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Shalai, Voice of Plenty",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Vision of Ixidor",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shalai, Voice of Plenty",
          "power": 3,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": null
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma, Vision of Ixidor",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Shalai, Voice of Plenty",
          "power": 4,
          "toughness": 5
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Balduvian Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shalai, Voice of Plenty",
          "power": 3,
          "toughness": 4
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});

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
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Akroma, Vision of Ixidor"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Shalai, Voice of Plenty"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 1,
          "toughness": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": null
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Akroma, Vision of Ixidor"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 0,
          "toughness": "Shalai, Voice of Plenty"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 1,
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": "end boost on end turn",
          "name": 2,
          "power": "PRECOMBAT_MAIN",
          "toughness": 0
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

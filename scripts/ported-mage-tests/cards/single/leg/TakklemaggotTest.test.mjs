import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/leg/TakklemaggotTest.java",
  "tests": [
    {
      "name": "testTakklemaggot",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Takklemaggot",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Harvest Hand",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "White Knight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mist Leopard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Takklemaggot",
          "target": "Harvest Hand"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Harvest Hand",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Harvest Hand",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Harvest Hand",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Mist Leopard"
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Scrounged Scythe",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Takklemaggot",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mist Leopard",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mist Leopard",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 7,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mist Leopard",
          "power": 3,
          "toughness": 1
        },
        {
          "op": "assertGraveyardCount",
          "turn": 8,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Mist Leopard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"t9\", 9, PhaseStep.PRECOMBAT_MAIN, playerB, 20)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"t10\", 10, PhaseStep.PRECOMBAT_MAIN, playerB, 19)"
        },
        {
          "op": "unsupported",
          "source": "checkLife(\"t11\", 11, PhaseStep.PRECOMBAT_MAIN, playerB, 19)"
        },
        {
          "op": "setStopAt",
          "turn": 12,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});

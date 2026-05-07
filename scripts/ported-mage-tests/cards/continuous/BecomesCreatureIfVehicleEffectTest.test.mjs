import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesCreatureIfVehicleEffectTest.java",
  "tests": [
    {
      "name": "testBecomesCreatureIfVehicleEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goliath Truck",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aerial Modification",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "checkType(\"Goliath Truck is not a creature\", 1, PhaseStep.PRECOMBAT_MAIN, playerA, goliathTruck, CardType.CREATURE, false)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aerial Modification",
          "target": "Goliath Truck"
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
          "op": "unsupported",
          "source": "assertType(goliathTruck, CardType.CREATURE, true)"
        }
      ]
    }
  ]
});

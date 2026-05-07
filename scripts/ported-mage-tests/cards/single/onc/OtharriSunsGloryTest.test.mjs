import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/onc/OtharriSunsGloryTest.java",
  "tests": [
    {
      "name": "test_Experience_Rebels",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Otharri, Suns' Glory",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Otharri, Suns' Glory",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Otharri, Suns' Glory",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "20 + 3 * 2"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "20 - 3 * 2 - 2 * (1 + 2)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rebel Token",
          "count": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "EXPERIENCE",
          "count": 2
        }
      ]
    }
  ]
});

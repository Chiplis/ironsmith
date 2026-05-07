import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/LegendRuleTest.java",
  "tests": [
    {
      "name": "testRegular",
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
          "zone": "HAND",
          "player": 0,
          "name": "Isamaru, Hound of Konda",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isamaru, Hound of Konda"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Isamaru, Hound of Konda"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Isamaru, Hound of Konda"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Isamaru, Hound of Konda",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Isamaru, Hound of Konda",
          "count": 1
        }
      ]
    }
  ]
});

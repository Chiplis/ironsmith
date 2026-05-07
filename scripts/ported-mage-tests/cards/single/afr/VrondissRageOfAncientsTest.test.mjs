import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afr/VrondissRageOfAncientsTest.java",
  "tests": [
    {
      "name": "testChaosDragonInteraction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vrondiss, Rage of Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chaos Dragon",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Chaos Dragon",
          "defender": 1
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 10)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerB, 11)"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerD, 13)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "No"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});

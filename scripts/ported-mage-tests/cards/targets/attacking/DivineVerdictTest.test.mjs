import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/targets/attacking/DivineVerdictTest.java",
  "tests": [
    {
      "name": "testAfterAttack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Divine Verdict",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Sejiri Merfolk",
          "defender": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Divine",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    }
  ]
});
